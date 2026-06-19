//! MCP command group
//!
//! `rustapi mcp generate` - Turn any OpenAPI 3.x spec into a running MCP server.
//! Tool calls are proxied to the real backend API.

use anyhow::{Context, Result};
use clap::Args;

#[cfg(feature = "mcp")]
use rustapi_mcp::{McpConfig, McpServer};
#[cfg(feature = "mcp")]
use rustapi_openapi::OpenApiSpec;

/// Arguments for `rustapi mcp generate`
#[derive(Args, Debug)]
pub struct McpGenerateArgs {
    /// Path to an OpenAPI spec file (JSON or YAML)
    #[arg(long, value_name = "FILE", conflicts_with_all = ["url", "api"])]
    pub spec: Option<String>,

    /// URL to fetch the OpenAPI spec from
    #[arg(long, value_name = "URL", conflicts_with_all = ["spec", "api"])]
    pub url: Option<String>,

    /// Base URL of a running service. Will try to fetch <base>/openapi.json
    /// and use the base as the proxy target.
    ///
    /// If the server is not running (or doesn't serve /openapi.json yet),
    /// prefer running `cargo rustapi mcp generate` with no flags inside your
    /// RustAPI project — it will auto-generate the spec directly from source.
    #[arg(long, value_name = "URL", conflicts_with_all = ["spec", "url"])]
    pub api: Option<String>,

    /// Target backend that MCP `tools/call` should proxy to
    /// (e.g. http://localhost:8000). Required unless --api is supplied.
    #[arg(long, value_name = "URL")]
    pub target: Option<String>,

    /// Port the MCP server will listen on (for agents / Claude Desktop etc.)
    #[arg(long, default_value_t = 9090)]
    pub port: u16,

    /// Human name for the MCP server (shown to LLM clients)
    #[arg(long)]
    pub name: Option<String>,

    /// Comma-separated list of tags. Only operations carrying at least one
    /// of these tags will be exposed as MCP tools.
    #[arg(long, value_name = "TAGS")]
    pub tags: Option<String>,

    /// Only expose paths that start with this prefix (e.g. "/api/v1")
    #[arg(long, value_name = "PREFIX")]
    pub allow_path_prefix: Option<String>,

    /// Use stdio transport instead of HTTP.
    ///
    /// This is useful for local AI clients (e.g. Claude Desktop) that speak
    /// MCP over standard input/output.
    #[arg(long)]
    pub stdio: bool,
}

/// Execute `rustapi mcp generate`
pub async fn mcp_generate(args: McpGenerateArgs) -> Result<()> {
    #[cfg(not(feature = "mcp"))]
    {
        anyhow::bail!(
            "MCP support is not enabled in this build of cargo-rustapi.\n\
             Rebuild with the 'mcp' feature or use a build that includes it."
        );
    }

    #[cfg(feature = "mcp")]
    {
        println!("🧠  RustAPI MCP generator");

        let spec_input = resolve_spec_source(&args)?;
        let spec_input = if spec_input == "__AUTO_GENERATE__" {
            println!("    No --spec/--url/--api given. Auto-generating OpenAPI spec from current project...");
            auto_generate_and_get_spec_path().await?
        } else {
            println!("    Loading OpenAPI spec...");
            spec_input
        };

        let openapi: OpenApiSpec = load_openapi_spec(&spec_input)
            .await
            .with_context(|| format!("Failed to load OpenAPI spec from {}", spec_input))?;

        let target = resolve_target(&args)?;

        let mut config = McpConfig::new();

        if let Some(name) = &args.name {
            config = config.name(name.clone());
        } else {
            // Derive a reasonable default name from the OpenAPI title
            let title = &openapi.info.title;
            config = config.name(format!("{}-mcp", sanitize_name(title)));
        }

        if let Some(tags_str) = &args.tags {
            let tags: Vec<String> = tags_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !tags.is_empty() {
                config = config.allowed_tags(tags);
            }
        }

        if let Some(prefix) = &args.allow_path_prefix {
            config = config.allow_path_prefix(prefix.clone());
        }

        let mut mcp = McpServer::from_spec(config, &openapi);
        mcp = mcp.with_http_base(target.clone());

        let addr = format!("127.0.0.1:{}", args.port); // safer default for local tool

        println!("    ✓ Spec loaded");
        println!("    → Proxying tool calls to: {}", target);

        if args.stdio {
            println!("🧠 MCP stdio transport active. Waiting for JSON-RPC on stdin...");
            run_stdio(mcp).await?;
            return Ok(());
        }

        println!("    → MCP server listening on: http://{}", addr);
        println!();
        println!("Useful test commands:");
        println!(
            "  curl -X POST http://127.0.0.1:{} -H 'content-type: application/json' \\",
            args.port
        );
        println!("       -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}}'");
        println!();
        println!(
            "  curl -X POST http://127.0.0.1:{} -H 'content-type: application/json' \\",
            args.port
        );
        println!("       -d '{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}}'");
        println!();
        println!("Press Ctrl+C to stop.");

        let shutdown = async {
            let _ = tokio::signal::ctrl_c().await;
        };

        mcp.serve_with_shutdown(&addr, shutdown)
            .await
            .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

        Ok(())
    }
}

#[cfg(feature = "mcp")]
async fn run_stdio(mcp: rustapi_mcp::McpServer) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut writer = ::tokio::io::stdout();

    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let json: serde_json::Value = match serde_json::from_str(line.trim()) {
            Ok(v) => v,
            Err(e) => {
                let err = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("parse error: {}", e) }
                });
                let mut out = serde_json::to_vec(&err).unwrap();
                out.push(b'\n');
                let _ = writer.write_all(&out).await;
                let _ = writer.flush().await;
                continue;
            }
        };

        let id = json.get("id").cloned().unwrap_or(serde_json::Value::Null);
        let method = json.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let result_val = match method {
            "initialize" => {
                let init = mcp.initialize();
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": { "name": init.name, "version": init.version },
                    "capabilities": { "tools": {} }
                })
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
                    serde_json::json!({ "tools": tool_defs })
                }
                Err(e) => serde_json::json!({ "code": -32603, "message": e.to_string() }),
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

                let tool_req = rustapi_mcp::ToolCallRequest { name, arguments };

                match mcp.call_tool(tool_req).await {
                    Ok(resp) => {
                        let text = if resp.content.is_null() {
                            String::new()
                        } else if let Some(s) = resp.content.as_str() {
                            s.to_owned()
                        } else {
                            serde_json::to_string_pretty(&resp.content)
                                .unwrap_or_else(|_| resp.content.to_string())
                        };
                        serde_json::json!({
                            "content": [{ "type": "text", "text": text }],
                            "isError": resp.is_error
                        })
                    }
                    Err(e) => {
                        serde_json::json!({
                            "content": [{ "type": "text", "text": format!("Tool error: {}", e) }],
                            "isError": true
                        })
                    }
                }
            }
            _ => {
                serde_json::json!({ "error": { "code": -32601, "message": "method not found" } })
            }
        };

        let resp = if result_val.get("error").is_some() {
            serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": result_val["error"] })
        } else {
            serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result_val })
        };

        let mut buf = serde_json::to_vec(&resp).unwrap();
        buf.push(b'\n');
        writer.write_all(&buf).await?;
        writer.flush().await?;
    }

    Ok(())
}

fn resolve_spec_source(args: &McpGenerateArgs) -> Result<String> {
    if let Some(s) = &args.spec {
        return Ok(s.clone());
    }
    if let Some(u) = &args.url {
        return Ok(u.clone());
    }
    if let Some(a) = &args.api {
        let base = a.trim_end_matches('/');
        return Ok(format!("{}/openapi.json", base));
    }
    // No source given → auto-generate from current RustAPI project
    // This allows `cargo rustapi mcp generate` to work without a running server
    // or pre-existing openapi.json file.
    // We return a special marker; the caller will handle auto generation.
    Ok("__AUTO_GENERATE__".to_string())
}

fn resolve_target(args: &McpGenerateArgs) -> Result<String> {
    if let Some(t) = &args.target {
        return Ok(t.clone());
    }
    if let Some(a) = &args.api {
        return Ok(a.clone());
    }
    // Auto mode default: assume the user's API runs on the common dev port
    println!("    No --target provided. Defaulting target to http://localhost:8080 (you can override with --target)");
    Ok("http://localhost:8080".to_string())
}

async fn load_openapi_spec(source: &str) -> Result<OpenApiSpec> {
    let content = if source.starts_with("http://") || source.starts_with("https://") {
        // remote-spec is pulled in by the mcp feature
        reqwest::get(source)
            .await
            .with_context(|| format!(
                "Failed to fetch OpenAPI spec from {}\n\
                 \n\
                 Tip: If this is your own RustAPI project, try running without --api:\n\
                 \n  cargo rustapi mcp generate --stdio\n\
                 \n\
                 This will auto-generate the spec directly from your code (no running server needed).",
                source
            ))?
            .text()
            .await
            .context("Failed to read response body")?
    } else {
        tokio::fs::read_to_string(source)
            .await
            .with_context(|| format!("Failed to read spec file: {}", source))?
    };

    let lower = source.to_ascii_lowercase();
    let spec = if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        serde_yaml::from_str(&content).context("Failed to deserialize YAML OpenAPI spec")?
    } else if lower.ends_with(".json") {
        serde_json::from_str(&content).context("Failed to deserialize JSON OpenAPI spec")?
    } else {
        // Unknown extension — try JSON then YAML
        serde_json::from_str(&content)
            .or_else(|_| serde_yaml::from_str(&content))
            .context("Failed to parse spec as JSON or YAML OpenAPI document")?
    };

    Ok(spec)
}

fn sanitize_name(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
}

/// When user runs `cargo rustapi mcp generate` without --spec/--url/--api,
/// we auto-extract the OpenAPI by running the project with a special env var
/// that makes RustApi print the spec and exit (before binding any port).
async fn auto_generate_and_get_spec_path() -> Result<String> {
    println!("    Spawning `cargo run` with RUSTAPI_DUMP_OPENAPI=1 to extract spec (no server binding)...");

    let output = std::process::Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .env("RUSTAPI_DUMP_OPENAPI", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .output()
        .context("Failed to execute `cargo run` for spec dump. Are you inside a RustAPI project?")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to build/run the project to extract OpenAPI.\n\
             Try running `cargo run` manually first to ensure it compiles, then retry `cargo rustapi mcp generate`."
        );
    }

    let stdout = String::from_utf8(output.stdout)
        .context("Captured OpenAPI output was not valid UTF-8")?;

    // The dump prints the JSON (possibly after some startup prints from the app).
    // Find the last occurrence of a top-level OpenAPI object for robustness.
    let json_str = if let Some(idx) = stdout.rfind(r#""openapi": "3."#) {
        // Go back to the opening { of that object
        let start = stdout[..idx].rfind('{').unwrap_or(0);
        let candidate = &stdout[start..];
        // cut at the last } 
        if let Some(end) = candidate.rfind('}') {
            candidate[..=end].trim().to_string()
        } else {
            candidate.trim().to_string()
        }
    } else if let Some(idx) = stdout.find('{') {
        let candidate = &stdout[idx..];
        candidate.trim().to_string()
    } else {
        stdout.trim().to_string()
    };

    if json_str.is_empty() || !json_str.starts_with('{') {
        anyhow::bail!(
            "Could not extract a valid OpenAPI JSON from the project dump.\n\
             Make sure your main uses RustApi::auto() or similar."
        );
    }

    // Write to a temp file so load_openapi_spec can handle it uniformly
    let temp_path = std::env::temp_dir().join(format!(
        "rustapi-auto-spec-{}.json",
        std::process::id()
    ));
    tokio::fs::write(&temp_path, &json_str)
        .await
        .context("Failed to write temp OpenAPI spec")?;

    println!("    ✓ Auto-generated spec written to temporary file");
    Ok(temp_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_produces_reasonable_name() {
        assert_eq!(sanitize_name("My Cool API!"), "my-cool-api");
    }

    #[tokio::test]
    async fn load_and_build_mcp_server_from_minimal_spec() {
        // A minimal but complete-enough OpenAPI that roundtrips through our deserializer
        let json = r#"{
            "openapi": "3.1.0",
            "info": {"title": "Test", "version": "1"},
            "paths": {
                "/ok": {
                    "get": {
                        "operationId": "okOp",
                        "tags": ["public"],
                        "responses": {
                            "200": {
                                "description": "ok",
                                "content": {"application/json": {"schema": {"type": "object"}}}
                            }
                        }
                    }
                }
            }
        }"#;

        let spec: OpenApiSpec = serde_json::from_str(json).expect("spec must deserialize");
        let cfg = McpConfig::new().allowed_tags(["public"]);
        let mcp = McpServer::from_spec(cfg, &spec);

        let tools = mcp.list_tools().await.expect("list_tools");
        assert!(
            !tools.is_empty(),
            "should have discovered at least one tool"
        );
        assert!(tools
            .iter()
            .any(|t| t.name.contains("ok") || t.name.contains("Op")));
    }
}
