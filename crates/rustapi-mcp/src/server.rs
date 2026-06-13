//! The main `McpServer` type and lifecycle.

use crate::config::McpConfig;
use crate::discovery;
use crate::error::{McpError, Result};
use crate::types::{McpCapability, McpTool};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::Response;
use hyper_util::rt::TokioIo;
use rustapi_core::RustApi;
use rustapi_openapi::OpenApiSpec;
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

/// The main handle for the MCP integration.
///
/// `McpServer` can be attached to a `RustApi` instance (via its OpenAPI spec)
/// to automatically discover tools. Tool invocations (in later milestones)
/// will be driven through the normal RustAPI handler pipeline.
#[derive(Debug, Clone)]
pub struct McpServer {
    config: Arc<McpConfig>,
    /// Attached OpenAPI spec used for tool discovery.
    openapi: Option<OpenApiSpec>,
    /// Base URL of the main RustAPI HTTP server (for proxy-style tool invocation over localhost).
    /// Example: "http://127.0.0.1:8080"
    http_base: Option<String>,
    /// Internal mapping from tool name (as advertised to MCP) to the original HTTP route info.
    /// Used to reconstruct the correct request when a tool is called.
    tool_map: HashMap<String, ToolExecutionInfo>,
}

/// Internal info needed to turn an MCP tool/call into an actual HTTP request
/// against the main API.
#[derive(Debug, Clone)]
struct ToolExecutionInfo {
    /// Original path template, e.g. "/users/{id}"
    path_template: String,
    /// HTTP method as string, e.g. "GET", "POST"
    method: String,
}

impl McpServer {
    /// Create a new MCP server with the given configuration.
    pub fn new(config: McpConfig) -> Self {
        Self {
            config: Arc::new(config),
            openapi: None,
            http_base: None,
            tool_map: HashMap::new(),
        }
    }

    /// Create an MCP server pre-attached to a `RustApi` instance.
    ///
    /// This is the most ergonomic way when you already have a built `RustApi`.
    pub fn from_rustapi(app: &RustApi, config: McpConfig) -> Self {
        let mut server = Self::new(config);
        let spec = app.openapi_spec().clone();
        server.openapi = Some(spec.clone());
        server.rebuild_tool_map(&spec);
        server
    }

    /// Create from an explicit OpenAPI spec.
    pub fn from_spec(config: McpConfig, spec: &OpenApiSpec) -> Self {
        let mut server = Self::new(config);
        server.openapi = Some(spec.clone());
        server.rebuild_tool_map(spec);
        server
    }

    /// Attach (or replace) the OpenAPI spec used for discovery.
    ///
    /// Call this after `RustApi::auto()` / builder if you built the app first.
    pub fn with_openapi(mut self, spec: OpenApiSpec) -> Self {
        self.rebuild_tool_map(&spec);
        self.openapi = Some(spec);
        self
    }

    /// Configure the base URL of the main RustAPI HTTP server.
    ///
    /// When set, `tools/call` will proxy the call over HTTP to this base
    /// (typically "http://127.0.0.1:8080" when using the concurrent runner).
    /// This guarantees that tool invocations go through the exact same
    /// middleware, auth, validation, and handler code as normal traffic.
    pub fn with_http_base(mut self, base: impl Into<String>) -> Self {
        self.http_base = Some(base.into());
        self
    }

    /// Rebuild the internal tool_name -> execution info map from the OpenAPI spec.
    /// Called automatically when attaching a spec.
    fn rebuild_tool_map(&mut self, spec: &OpenApiSpec) {
        self.tool_map.clear();

        let config = &*self.config;

        let insert_tool = |map: &mut HashMap<String, ToolExecutionInfo>,
                           path: &str,
                           method: &str,
                           op: &rustapi_openapi::Operation| {
            if !config.allowed_tags.is_empty() {
                let has_match = op.tags.iter().any(|t| config.allowed_tags.contains(t));
                if !has_match {
                    return;
                }
            }

            let name = generate_tool_name(method, path, op);
            let info = ToolExecutionInfo {
                path_template: path.to_string(),
                method: method.to_string(),
            };
            map.insert(name, info);
        };

        for (path, path_item) in &spec.paths {
            if !path_matches_prefixes(path, &config.allowed_path_prefixes) {
                continue;
            }

            if let Some(op) = &path_item.get {
                insert_tool(&mut self.tool_map, path, "GET", op);
            }
            if let Some(op) = &path_item.post {
                insert_tool(&mut self.tool_map, path, "POST", op);
            }
            if let Some(op) = &path_item.put {
                insert_tool(&mut self.tool_map, path, "PUT", op);
            }
            if let Some(op) = &path_item.patch {
                insert_tool(&mut self.tool_map, path, "PATCH", op);
            }
            if let Some(op) = &path_item.delete {
                insert_tool(&mut self.tool_map, path, "DELETE", op);
            }
        }
    }

    /// Get a reference to the active configuration.
    pub fn config(&self) -> &McpConfig {
        &self.config
    }

    /// Return the capabilities this server currently advertises.
    pub fn capabilities(&self) -> Vec<McpCapability> {
        let mut caps = vec![];

        if self.config.tools_enabled {
            caps.push(McpCapability::Tools);
        }

        caps
    }

    /// Perform the `initialize` handshake.
    ///
    /// MCP clients call this first. Returns server info + supported capabilities.
    pub fn initialize(&self) -> InitializeResult {
        InitializeResult {
            name: self.config.name.clone(),
            version: self.config.version.clone(),
            description: self.config.description.clone(),
            capabilities: self.capabilities(),
        }
    }

    /// Discover and return the list of tools that should be exposed to MCP clients.
    ///
    /// Tools are derived from the attached OpenAPI spec (from `RustApi`), filtered
    /// according to `McpConfig::allowed_tags` and `allowed_path_prefixes`.
    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        if !self.config.tools_enabled {
            return Err(crate::error::McpError::CapabilityNotEnabled(
                "tools".to_string(),
            ));
        }

        if let Some(spec) = &self.openapi {
            let tools = discovery::extract_tools_from_spec(spec, &self.config);
            Ok(tools)
        } else {
            // No spec attached → no tools (safe default)
            Ok(vec![])
        }
    }

    /// Execute a tool call by proxying it as a real HTTP request to the main
    /// RustAPI server (using the configured `http_base`).
    ///
    /// This is the heart of Native MCP: the call goes through your normal
    /// layers, interceptors, extractors, validation, error handling, etc.
    pub async fn call_tool(
        &self,
        req: crate::types::ToolCallRequest,
    ) -> Result<crate::types::ToolCallResponse> {
        let info = self.tool_map.get(&req.name).ok_or_else(|| {
            McpError::ToolNotFound(format!("no tool registered with name '{}'", req.name))
        })?;

        let path = substitute_path_params(&info.path_template, &req.arguments);

        let base = self.http_base.as_deref().unwrap_or("http://127.0.0.1:8080");

        let url = format!("{}{}", base.trim_end_matches('/'), path);

        let method =
            reqwest::Method::from_bytes(info.method.as_bytes()).unwrap_or(reqwest::Method::GET);

        let client = reqwest::Client::new();

        let mut request_builder = client.request(method.clone(), &url);

        // If this looks like a mutating method, send the arguments as JSON body
        let is_body_method = matches!(info.method.as_str(), "POST" | "PUT" | "PATCH");
        if is_body_method && !req.arguments.is_empty() {
            request_builder = request_builder
                .header("content-type", "application/json")
                .json(&req.arguments);
        } else if !is_body_method && !req.arguments.is_empty() {
            // For GET/DELETE etc, we could turn remaining args into query params.
            // For MVP we rely on path params; extra args are ignored for now.
        }

        let resp = request_builder.send().await.map_err(|e| {
            McpError::ToolExecution(format!("failed to proxy tool call to main API: {}", e))
        })?;

        let status = resp.status();
        let is_error = !status.is_success();

        let content = if let Ok(text) = resp.text().await {
            if text.trim().is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
            }
        } else {
            serde_json::Value::Null
        };

        Ok(crate::types::ToolCallResponse {
            content,
            is_error,
            meta: Some(serde_json::json!({ "proxied_status": status.as_u16() })),
        })
    }
}

/// Result of the `initialize` handshake.
#[derive(Debug, Clone)]
pub struct InitializeResult {
    /// Server name (from config).
    pub name: String,
    /// Server version (from config).
    pub version: String,
    /// Optional description.
    pub description: Option<String>,
    /// Advertised capabilities.
    pub capabilities: Vec<McpCapability>,
}

impl McpServer {
    /// Start serving the MCP protocol over HTTP on the given address.
    ///
    /// This starts a sidecar HTTP server (separate from your main RustAPI HTTP server)
    /// that MCP clients (Claude, etc.) can connect to for tool discovery and invocation.
    ///
    /// Supports a minimal JSON-RPC over POST transport for:
    /// - `initialize`
    /// - `tools/list`
    /// - `tools/call` (proxies to the main RustAPI server so full middleware / validation / error handling applies)
    pub async fn serve(self, addr: &str) -> Result<()> {
        self.serve_with_shutdown(addr, std::future::pending()).await
    }

    /// Like `serve`, but with a shutdown signal (e.g. ctrl_c()).
    pub async fn serve_with_shutdown<F>(self, addr: &str, signal: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|e| McpError::Transport(format!("invalid MCP address '{}': {}", addr, e)))?;

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            McpError::Transport(format!("failed to bind MCP listener on {}: {}", addr, e))
        })?;

        info!("🧠 MCP server listening on http://{}", addr);

        let this = Arc::new(self);
        tokio::pin!(signal);

        loop {
            tokio::select! {
                biased;

                accept_result = listener.accept() => {
                    let (stream, remote) = match accept_result {
                        Ok(v) => v,
                        Err(e) => {
                            error!("MCP accept error: {}", e);
                            continue;
                        }
                    };

                    let io = TokioIo::new(stream);
                    let mcp = this.clone();

                    tokio::task::spawn(async move {
                        let service = hyper::service::service_fn(move |req| {
                            let mcp = mcp.clone();
                            async move { handle_mcp_http_request(mcp, req).await }
                        });

                        if let Err(e) = http1::Builder::new()
                            .serve_connection(io, service)
                            .await
                        {
                            error!("MCP connection error from {}: {}", remote, e);
                        }
                    });
                }

                _ = &mut signal => {
                    info!("MCP server received shutdown signal");
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Handle a single HTTP request for the MCP protocol (minimal JSON-RPC over POST).
async fn handle_mcp_http_request(
    mcp: Arc<McpServer>,
    req: hyper::Request<Incoming>,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    if req.method() != hyper::Method::POST {
        let body = Bytes::from_static(b"{\"error\":\"MCP transport expects POST requests\"}");
        return Ok(Response::builder()
            .status(405)
            .header("content-type", "application/json")
            .body(Full::new(body))
            .expect("static response must build"));
    }

    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            let err_body = format!("{{\"error\":\"failed to read body: {}\"}}", e);
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(Full::new(Bytes::from(err_body)))
                .expect("error response must build"));
        }
    };

    let json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(_) => {
            return Ok(jsonrpc_error_response(
                serde_json::Value::Null,
                -32700,
                "parse error",
            ));
        }
    };

    let id = json.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let method = json.get("method").and_then(|m| m.as_str()).unwrap_or("");

    match method {
        "initialize" => {
            let init = mcp.initialize();
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": init.name,
                    "version": init.version
                },
                "capabilities": {
                    "tools": {}
                }
            });
            Ok(jsonrpc_success_response(id, result))
        }
        "tools/list" => match mcp.list_tools().await {
            Ok(tools) => {
                let tool_defs: Vec<_> = tools
                    .into_iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "inputSchema": t.input_schema
                        })
                    })
                    .collect();

                let result = serde_json::json!({ "tools": tool_defs });
                Ok(jsonrpc_success_response(id, result))
            }
            Err(e) => Ok(jsonrpc_error_response(
                id,
                -32603,
                &format!("internal error: {}", e),
            )),
        },
        "tools/call" => {
            let params = json.get("params").cloned().unwrap_or(serde_json::json!({}));
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let arguments: std::collections::HashMap<String, serde_json::Value> = params
                .get("arguments")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let tool_req = crate::types::ToolCallRequest { name, arguments };

            match mcp.call_tool(tool_req).await {
                Ok(resp) => {
                    // Shape the response per MCP tools/call convention (protocol 2024-11-05):
                    // The result contains a "content" array of content blocks + "isError" flag.
                    // We serialize non-string content as pretty JSON text for broad client compatibility.
                    let text = if resp.content.is_null() {
                        String::new()
                    } else if let Some(s) = resp.content.as_str() {
                        s.to_owned()
                    } else {
                        serde_json::to_string_pretty(&resp.content)
                            .unwrap_or_else(|_| resp.content.to_string())
                    };

                    let result = serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": text
                        }],
                        "isError": resp.is_error
                    });
                    Ok(jsonrpc_success_response(id, result))
                }
                Err(e) => {
                    // Surface execution errors as a successful JSON-RPC but with isError inside the result
                    // (this is the MCP convention so the client knows it was a tool-level failure).
                    let result = serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Tool execution error: {}", e)
                        }],
                        "isError": true
                    });
                    Ok(jsonrpc_success_response(id, result))
                }
            }
        }
        _ => Ok(jsonrpc_error_response(id, -32601, "method not found")),
    }
}

fn jsonrpc_success_response(
    id: serde_json::Value,
    result: serde_json::Value,
) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    });
    Response::builder()
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&body).expect("json must serialize"),
        )))
        .expect("response must build")
}

fn jsonrpc_error_response(
    id: serde_json::Value,
    code: i32,
    message: &str,
) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    });
    Response::builder()
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&body).expect("json must serialize"),
        )))
        .expect("response must build")
}

/// Helpers duplicated from discovery for tool map building (small & self-contained).
fn path_matches_prefixes(path: &str, prefixes: &[String]) -> bool {
    if prefixes.is_empty() {
        return true;
    }
    prefixes.iter().any(|p| path.starts_with(p))
}

fn generate_tool_name(method: &str, path: &str, op: &rustapi_openapi::Operation) -> String {
    if let Some(oid) = &op.operation_id {
        return sanitize_name(oid);
    }
    let mut slug = path
        .trim_start_matches('/')
        .replace(['/', '{', '}', ':'], "_")
        .replace(['-', '.', ' '], "_");
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    let slug = slug.trim_matches('_').to_string();
    let method_lower = method.to_lowercase();
    if slug.is_empty() {
        method_lower
    } else {
        format!("{}_{}", method_lower, slug)
    }
}

fn sanitize_name(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
        .to_lowercase()
}

/// Substitute {param} placeholders in the path template using values from the MCP arguments.
fn substitute_path_params(
    template: &str,
    args: &std::collections::HashMap<String, serde_json::Value>,
) -> String {
    let mut result = template.to_string();
    for (key, value) in args {
        let placeholder = format!("{{{}}}", key);
        if result.contains(&placeholder) {
            let val_str = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string().trim_matches('"').to_string(),
            };
            result = result.replace(&placeholder, &val_str);
        }
    }
    result
}
