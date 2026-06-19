//! Core MCP data types (tool definitions, requests, responses, capabilities).
//!
//! These types are intentionally high-level for the foundation phase.
//! They will be expanded with full JSON-RPC message shapes when we implement transports.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A capability that this MCP server advertises during `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum McpCapability {
    /// The server can list and invoke tools.
    Tools,
    /// Future: resources, prompts, sampling, etc.
    #[serde(other)]
    Other,
}

/// A tool description that will be sent to MCP clients in `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    /// Stable name of the tool (usually derived from path + method or a slug).
    pub name: String,

    /// Human readable description (comes from OpenAPI `summary` / `description` when available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// JSON Schema for the input parameters (reused from `rustapi-openapi` / `Schema` derive).
    pub input_schema: serde_json::Value,

    /// Optional JSON Schema for the output (when we have good response schemas).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,

    /// Tags associated with this tool (used for filtering via `McpConfig`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Framework-level permission classification ("read" | "write").
    /// This is the key part of native scoping — agents can see the blast radius.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<String>,

    /// Whether this tool should trigger a confirmation prompt on the agent side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_confirmation: Option<bool>,
}

/// Request from an MCP client to call a tool.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallRequest {
    /// Name of the tool to invoke.
    pub name: String,
    /// Arguments passed to the tool (will be turned into extractors later).
    #[serde(default)]
    pub arguments: HashMap<String, serde_json::Value>,
}

/// Successful result of a tool call.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResponse {
    /// The actual content returned by the underlying handler (serialized appropriately).
    pub content: serde_json::Value,

    /// Whether this result should be treated as an error by the agent (even if HTTP 2xx).
    #[serde(default)]
    pub is_error: bool,

    /// Optional metadata (token counts when using TOON, execution path, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl ToolCallResponse {
    /// Construct a successful tool response.
    pub fn success(content: impl Serialize) -> Self {
        Self {
            content: serde_json::to_value(content).unwrap_or(serde_json::json!({})),
            is_error: false,
            meta: None,
        }
    }

    /// Construct an error tool response (from the agent's perspective).
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: serde_json::json!({ "error": message.into() }),
            is_error: true,
            meta: None,
        }
    }
}
