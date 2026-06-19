//! In-process request invocation for MCP (zero TCP overhead).
//!
//! When an MCP server is created via `from_rustapi`, we can drive tool calls
//! directly through the application's Router + LayerStack + Interceptors.

use crate::config::{InvocationMode, McpConfig};
use rustapi_core::{Request, RequestDispatcher, Response as CoreResponse};
use std::sync::Arc;

/// Executes tool calls directly against a RustAPI instance (in-process).
///
/// This bypasses the network entirely while still running the full pipeline:
/// request interceptors, middleware layers, route handler + extractors/validators,
/// response interceptors.
#[derive(Clone)]
pub struct RequestInvoker {
    dispatcher: Arc<RequestDispatcher>,
    config: Arc<McpConfig>,
}

impl std::fmt::Debug for RequestInvoker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestInvoker")
            .field("mode", &self.config.invocation_mode)
            .finish_non_exhaustive()
    }
}

impl RequestInvoker {
    pub(crate) fn new(dispatcher: RequestDispatcher, config: Arc<McpConfig>) -> Self {
        Self {
            dispatcher: Arc::new(dispatcher),
            config,
        }
    }

    /// Perform an in-process tool call.
    ///
    /// The caller is responsible for having already looked up the route info
    /// and built a proper `core::Request`.
    pub async fn invoke(&self, req: Request) -> CoreResponse {
        self.dispatcher.dispatch(req).await
    }

    /// Whether this invoker should be used based on the current mode.
    pub fn should_use(&self) -> bool {
        match self.config.invocation_mode {
            InvocationMode::Proxy => false,
            InvocationMode::InProcess | InvocationMode::Auto => true,
        }
    }
}
