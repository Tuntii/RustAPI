//! MCP-specific error types.

use thiserror::Error;

/// Result alias used throughout the `rustapi-mcp` crate.
pub type Result<T> = std::result::Result<T, McpError>;

/// Top-level error type for MCP operations.
#[derive(Debug, Error)]
pub enum McpError {
    /// The MCP client is not authorized (missing or bad token).
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// The requested capability (e.g. tools) is not enabled in config.
    #[error("capability not enabled: {0}")]
    CapabilityNotEnabled(String),

    /// A tool with the given name was not found / not exposed.
    #[error("tool not found: {0}")]
    ToolNotFound(String),

    /// Invalid request from the MCP client (bad parameters, schema mismatch, etc.).
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// An error occurred while executing the underlying RustAPI handler.
    /// The inner value is the stringified error (respecting redaction rules).
    #[error("tool execution failed: {0}")]
    ToolExecution(String),

    /// Transport-level or protocol-level error.
    #[error("transport error: {0}")]
    Transport(String),

    /// Internal / unexpected error.
    #[error("internal mcp error: {0}")]
    Internal(String),
}

impl McpError {
    /// Create an unauthorized error.
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        McpError::Unauthorized(msg.into())
    }

    /// Create an invalid request error.
    pub fn invalid_request(msg: impl Into<String>) -> Self {
        McpError::InvalidRequest(msg.into())
    }
}
