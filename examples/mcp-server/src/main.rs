//! MCP (Model Context Protocol) Server Example
//!
//! This example demonstrates a simple MCP server implementation using RustAPI.
//! MCP servers expose tools, resources, and prompts that AI assistants can use.
//!
//! ## Features
//! - TOON format for 50-58% token savings
//! - Automatic token counting headers
//! - Content negotiation (JSON/TOON)
//! - Type-safe tool definitions
//!
//! ## Running
//!
//! ```bash
//! cargo run --example mcp-server
//! ```
//!
//! ## Testing
//!
//! ### List available tools:
//! ```bash
//! curl http://localhost:8080/mcp/tools
//! ```
//!
//! ### List with TOON format (for LLMs):
//! ```bash
//! curl -H "Accept: application/toon" http://localhost:8080/mcp/tools
//! ```
//!
//! ### Execute a tool:
//! ```bash
//! curl -X POST http://localhost:8080/mcp/tools/execute \
//!   -H "Content-Type: application/json" \
//!   -d '{
//!     "tool": "calculate",
//!     "arguments": {
//!       "operation": "add",
//!       "a": 5,
//!       "b": 3
//!     }
//!   }'
//! ```
//!
//! ### List resources:
//! ```bash
//! curl http://localhost:8080/mcp/resources
//! ```

use rustapi_rs::prelude::*;
use rustapi_rs::toon::{AcceptHeader, LlmResponse, Toon};
use std::collections::HashMap;
use utoipa::ToSchema;

// --- Data Models ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct Tool {
    name: String,
    description: String,
    input_schema: ToolSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct ToolSchema {
    #[serde(rename = "type")]
    schema_type: String,
    properties: HashMap<String, PropertySchema>,
    required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct PropertySchema {
    #[serde(rename = "type")]
    prop_type: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct ToolsListResponse {
    tools: Vec<Tool>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct ToolExecuteRequest {
    tool: String,
    arguments: HashMap<String, String>, // Simplified to String instead of serde_json::Value
}

#[derive(Debug, Clone, Serialize, ToSchema)]
struct ToolExecuteResponse {
    success: bool,
    result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct Resource {
    uri: String,
    name: String,
    description: String,
    mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct ResourcesListResponse {
    resources: Vec<Resource>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
struct ServerInfo {
    name: String,
    version: String,
    protocol_version: String,
    capabilities: Capabilities,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
struct Capabilities {
    tools: bool,
    resources: bool,
    prompts: bool,
}

// --- MCP Handlers ---

/// Get server information
#[rustapi_rs::get("/mcp")]
#[rustapi_rs::tag("MCP")]
#[rustapi_rs::summary("Get MCP Server Info")]
async fn get_server_info() -> Json<ServerInfo> {
    Json(ServerInfo {
        name: "RustAPI MCP Server".to_string(),
        version: "0.1.0".to_string(),
        protocol_version: "2024-11-05".to_string(),
        capabilities: Capabilities {
            tools: true,
            resources: true,
            prompts: false,
        },
    })
}

/// List available tools (with automatic JSON/TOON negotiation)
#[rustapi_rs::get("/mcp/tools")]
#[rustapi_rs::tag("MCP")]
#[rustapi_rs::summary("List Available Tools")]
async fn list_tools(accept: AcceptHeader) -> LlmResponse<ToolsListResponse> {
    let tools = vec![
        Tool {
            name: "calculate".to_string(),
            description: "Perform basic arithmetic operations".to_string(),
            input_schema: ToolSchema {
                schema_type: "object".to_string(),
                properties: HashMap::from([
                    (
                        "operation".to_string(),
                        PropertySchema {
                            prop_type: "string".to_string(),
                            description: "The operation to perform".to_string(),
                            enum_values: Some(vec![
                                "add".to_string(),
                                "subtract".to_string(),
                                "multiply".to_string(),
                                "divide".to_string(),
                            ]),
                        },
                    ),
                    (
                        "a".to_string(),
                        PropertySchema {
                            prop_type: "number".to_string(),
                            description: "First operand".to_string(),
                            enum_values: None,
                        },
                    ),
                    (
                        "b".to_string(),
                        PropertySchema {
                            prop_type: "number".to_string(),
                            description: "Second operand".to_string(),
                            enum_values: None,
                        },
                    ),
                ]),
                required: vec!["operation".to_string(), "a".to_string(), "b".to_string()],
            },
        },
        Tool {
            name: "get_weather".to_string(),
            description: "Get current weather for a location".to_string(),
            input_schema: ToolSchema {
                schema_type: "object".to_string(),
                properties: HashMap::from([
                    (
                        "location".to_string(),
                        PropertySchema {
                            prop_type: "string".to_string(),
                            description: "City name or coordinates".to_string(),
                            enum_values: None,
                        },
                    ),
                    (
                        "units".to_string(),
                        PropertySchema {
                            prop_type: "string".to_string(),
                            description: "Temperature units".to_string(),
                            enum_values: Some(vec![
                                "celsius".to_string(),
                                "fahrenheit".to_string(),
                            ]),
                        },
                    ),
                ]),
                required: vec!["location".to_string()],
            },
        },
    ];

    LlmResponse::new(ToolsListResponse { tools }, accept.preferred)
}

/// Execute a tool
#[rustapi_rs::post("/mcp/tools/execute")]
#[rustapi_rs::tag("MCP")]
#[rustapi_rs::summary("Execute a Tool")]
async fn execute_tool(Json(request): Json<ToolExecuteRequest>) -> Toon<ToolExecuteResponse> {
    match request.tool.as_str() {
        "calculate" => {
            let operation = request
                .arguments
                .get("operation")
                .map(|v| v.as_str())
                .unwrap_or("add");
            let a = request
                .arguments
                .get("a")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            let b = request
                .arguments
                .get("b")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);

            let result = match operation {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b == 0.0 {
                        return Toon(ToolExecuteResponse {
                            success: false,
                            result: String::new(),
                            error: Some("Division by zero".to_string()),
                        });
                    }
                    a / b
                }
                _ => {
                    return Toon(ToolExecuteResponse {
                        success: false,
                        result: String::new(),
                        error: Some(format!("Unknown operation: {}", operation)),
                    });
                }
            };

            Toon(ToolExecuteResponse {
                success: true,
                result: format!("{} {} {} = {}", a, operation, b, result),
                error: None,
            })
        }
        "get_weather" => {
            let location = request
                .arguments
                .get("location")
                .map(|v| v.as_str())
                .unwrap_or("Unknown");
            let units = request
                .arguments
                .get("units")
                .map(|v| v.as_str())
                .unwrap_or("celsius");

            Toon(ToolExecuteResponse {
                success: true,
                result: format!(
                    "Weather in {}: 22°{} (Sunny)",
                    location,
                    if units == "fahrenheit" { "F" } else { "C" }
                ),
                error: None,
            })
        }
        _ => Toon(ToolExecuteResponse {
            success: false,
            result: String::new(),
            error: Some(format!("Unknown tool: {}", request.tool)),
        }),
    }
}

/// List available resources
#[rustapi_rs::get("/mcp/resources")]
#[rustapi_rs::tag("MCP")]
#[rustapi_rs::summary("List Available Resources")]
async fn list_resources(accept: AcceptHeader) -> LlmResponse<ResourcesListResponse> {
    let resources = vec![
        Resource {
            uri: "resource://docs/getting-started".to_string(),
            name: "Getting Started Guide".to_string(),
            description: "Introduction to the MCP server".to_string(),
            mime_type: "text/markdown".to_string(),
        },
        Resource {
            uri: "resource://docs/api-reference".to_string(),
            name: "API Reference".to_string(),
            description: "Complete API documentation".to_string(),
            mime_type: "text/markdown".to_string(),
        },
    ];

    LlmResponse::new(ResourcesListResponse { resources }, accept.preferred)
}

/// Health check endpoint
#[rustapi_rs::get("/health")]
#[rustapi_rs::tag("Health")]
#[rustapi_rs::summary("Health Check")]
async fn health_check() -> &'static str {
    "OK"
}

// --- Main ---

#[tokio::main]
async fn main() {
    println!("🚀 MCP Server starting...");
    println!("📖 Swagger UI: http://localhost:8080/docs");
    println!("🤖 MCP Info: http://localhost:8080/mcp");
    println!("🔧 Tools: http://localhost:8080/mcp/tools");
    println!("📦 Resources: http://localhost:8080/mcp/resources");
    println!("\n💡 Tip: Use 'Accept: application/toon' header for LLM-optimized responses\n");

    let _ = RustApi::auto().run("127.0.0.1:8080").await;
}
