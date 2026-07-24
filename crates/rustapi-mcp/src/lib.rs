//! # rustapi-mcp
//!
//! Native [Model Context Protocol (MCP)](https://modelcontextprotocol.io) support for RustAPI.
//!
//! This crate turns your RustAPI application into a first-class tool provider for LLMs
//! and external AI agents (Claude, custom agent runtimes, etc.).
//!
//! ## Philosophy
//!
//! - **Embedded & opt-in**: Enable with the `protocol-mcp` feature.
//! - **Zero duplication**: Tool definitions are derived from your existing routes,
//!   `#[derive(Schema)]` types, and OpenAPI metadata.
//! - **Security first**: Nothing is exposed as a tool unless you explicitly allow it
//!   (tags, paths, or manual registration). Destructive operations are hidden by default via
//!   `ToolPolicy::ReadOnly`.
//! - **Respect the pipeline**: Every tool invocation goes through your normal middleware,
//!   interceptors, extractors, validation, and error handling. No secret bypass paths.
//! - **Permission metadata**: Tools declare "read" vs "write" and whether confirmation is needed.
//!
//! ## Current Status
//!
//! **Native MCP is implemented and functional** (discovery + real invocation + transport + concurrent runner).
//!
//! - Automatic tool discovery from your `#[rustapi_rs::get(...)]` routes + `#[derive(Schema)]` via OpenAPI.
//! - Full respect for tags (`allowed_tags`) and path prefixes for safe exposure.
//! - Framework-native permission scoping (`ToolPolicy::ReadOnly` default, `#[mcp(skip)]`, `#[mcp(write, require="confirm")]`).
//! - Sidecar HTTP server speaking minimal MCP JSON-RPC (initialize, tools/list, tools/call).
//! - Optional `admin_token`: when set, HTTP clients must present `Authorization: Bearer`,
//!   `X-MCP-Token`, or `?token=` (fail closed with HTTP 401).
//! - Safe-method tool args not used in the path template become query parameters.
//! - Real `tools/call` execution: proxy to the main HTTP API (default) or in-process
//!   via `InvocationMode::InProcess` / `Auto` when created with `from_rustapi`.
//! - `run_rustapi_and_mcp` (and with shutdown) helpers to run your API + MCP endpoint side-by-side (auto-configures proxying).
//!
//! See `memories/native_mcp_orchestration_plan.md` for the original roadmap.
//!
//! ## Quick Example
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_rs::protocol::mcp::{McpConfig, McpServer, run_rustapi_and_mcp};
//!
//! #[rustapi_rs::get("/weather/{city}")]
//! #[rustapi_rs::tag("public")]
//! async fn get_weather(Path(city): Path<String>) -> Json<Weather> {
//!     // ...
//! }
//!
//! let app = RustApi::auto();
//!
//! let mcp = McpServer::from_rustapi(
//!     &app,
//!     McpConfig::new()
//!         .name("weather-agent")
//!         .allowed_tags(["public"]),
//! );
//!
//! // Runs your normal HTTP API on :8080 and MCP server on :9090.
//! // tool calls are automatically proxied back into the main API (full stack).
//! run_rustapi_and_mcp(app, "0.0.0.0:8080", mcp, "0.0.0.0:9090").await?;
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![deny(clippy::unwrap_used)] // encourage proper error handling from day one

pub mod config;
pub(crate) mod discovery;
pub mod error;
pub mod runner;
pub mod server;
pub mod types;

// Re-export the most important items at the crate root for convenience.
pub use config::{InvocationMode, McpConfig, ToolPolicy};
pub use error::{McpError, Result};
pub use runner::{
    run_concurrently, run_rustapi_and_mcp, run_rustapi_and_mcp_with_shutdown, BoxError,
};
pub use server::McpServer;
pub use types::{McpCapability, McpTool, ToolCallRequest, ToolCallResponse};

// Re-export OpenApiSpec for ergonomic attachment: users can pass app.openapi_spec().clone()
pub use rustapi_openapi::OpenApiSpec;

/// Prelude for common MCP types.
pub mod prelude {
    pub use crate::config::{InvocationMode, McpConfig, ToolPolicy};
    pub use crate::error::{McpError, Result};
    pub use crate::runner::{
        run_concurrently, run_rustapi_and_mcp, run_rustapi_and_mcp_with_shutdown,
    };
    pub use crate::server::McpServer;
    pub use crate::types::{McpTool, ToolCallRequest, ToolCallResponse};
}

/// Internal module for future transport implementations (HTTP+SSE, stdio, etc.).
pub(crate) mod transport {
    // Will contain SSE framing, JSON-RPC handling, etc.
}

/// Internal helpers for executing tool calls through the normal RustAPI stack.
pub(crate) mod invocation;
