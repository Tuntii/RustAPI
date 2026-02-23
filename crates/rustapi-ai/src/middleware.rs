//! AI-Native HTTP middleware integration.
//!
//! This module provides the [`AiContextLayer`] middleware that automatically
//! creates a [`RequestContext`] for every incoming HTTP request and inserts it
//! (along with the [`AiRuntime`]) into the request extensions.
//!
//! Handlers can then extract the AI context with the [`AiCtx`] extractor:
//!
//! ```rust,ignore
//! use rustapi_ai::middleware::AiCtx;
//! use rustapi_context::RequestContext;
//!
//! async fn my_handler(AiCtx(ctx): AiCtx, AiRt(runtime): AiRt) -> String {
//!     let trace = ctx.trace();
//!     let _span = trace.start_span(
//!         rustapi_context::TraceNodeKind::AgentStep,
//!         "my_handler",
//!     );
//!     "OK".to_string()
//! }
//! ```

use crate::AiRuntime;
use rustapi_context::{CostBudget, RequestContext, RequestContextBuilder};
use rustapi_core::middleware::{BoxedNext, MiddlewareLayer};
use rustapi_core::{Request, Response};
use std::future::Future;
use std::pin::Pin;
use tracing::debug;

// ---------------------------------------------------------------------------
// AiContextLayer — middleware
// ---------------------------------------------------------------------------

/// Middleware that creates a [`RequestContext`] for every request and makes
/// the [`AiRuntime`] available to extractors.
///
/// # What it inserts into extensions
///
/// - [`RequestContext`] — per-request trace, cost, observability context
/// - [`AiRuntime`] — shared AI runtime (tools, memory, LLM router)
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_ai::{AiRuntime, middleware::AiContextLayer};
///
/// let runtime = AiRuntime::builder().build();
///
/// RustApi::new()
///     .layer(AiContextLayer::new(runtime))
///     .route("/chat", post(chat_handler));
/// ```
#[derive(Clone)]
pub struct AiContextLayer {
    runtime: AiRuntime,
    default_budget: Option<CostBudget>,
}

impl AiContextLayer {
    /// Create a new AI context middleware with the given runtime.
    pub fn new(runtime: AiRuntime) -> Self {
        Self {
            runtime,
            default_budget: None,
        }
    }

    /// Set a default cost budget applied to every request context.
    pub fn with_budget(mut self, budget: CostBudget) -> Self {
        self.default_budget = Some(budget);
        self
    }
}

impl MiddlewareLayer for AiContextLayer {
    fn call(
        &self,
        mut req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let runtime = self.runtime.clone();
        let budget = self.default_budget.clone();

        Box::pin(async move {
            // Build a fresh RequestContext per request
            let mut builder = RequestContextBuilder::new()
                .method(req.method().as_str())
                .path(req.path());

            // Apply default budget if configured
            if let Some(b) = budget {
                builder = builder.budget(b);
            }

            // Propagate x-request-id header as metadata
            if let Some(rid) = req
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
            {
                builder = builder.metadata(
                    "request_id",
                    serde_json::Value::String(rid.to_string()),
                );
            }

            let ctx = builder.build();

            debug!(context_id = %ctx.id(), "AI context created");

            // Insert into extensions for handler extraction
            req.extensions_mut().insert(ctx);
            req.extensions_mut().insert(runtime);

            next(req).await
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// AiCtx — extractor for RequestContext
// ---------------------------------------------------------------------------

/// Extractor that provides the [`RequestContext`] created by [`AiContextLayer`].
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_ai::middleware::AiCtx;
///
/// async fn handler(AiCtx(ctx): AiCtx) -> String {
///     format!("Request ID: {}", ctx.observability().request_id())
/// }
/// ```
pub struct AiCtx(pub RequestContext);

impl rustapi_core::FromRequestParts for AiCtx {
    fn from_request_parts(req: &Request) -> rustapi_core::Result<Self> {
        req.extensions()
            .get::<RequestContext>()
            .cloned()
            .map(AiCtx)
            .ok_or_else(|| {
                rustapi_core::ApiError::internal(
                    "RequestContext not found in extensions. \
                     Did you add AiContextLayer middleware?",
                )
            })
    }
}

// ---------------------------------------------------------------------------
// AiRt — extractor for AiRuntime
// ---------------------------------------------------------------------------

/// Extractor that provides the [`AiRuntime`] inserted by [`AiContextLayer`].
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_ai::middleware::{AiCtx, AiRt};
///
/// async fn handler(AiCtx(ctx): AiCtx, AiRt(rt): AiRt) -> String {
///     let engine = rt.create_engine(Default::default());
///     "OK".to_string()
/// }
/// ```
pub struct AiRt(pub AiRuntime);

impl rustapi_core::FromRequestParts for AiRt {
    fn from_request_parts(req: &Request) -> rustapi_core::Result<Self> {
        req.extensions()
            .get::<AiRuntime>()
            .cloned()
            .map(AiRt)
            .ok_or_else(|| {
                rustapi_core::ApiError::internal(
                    "AiRuntime not found in extensions. \
                     Did you add AiContextLayer middleware?",
                )
            })
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::Extensions;
    use rustapi_core::{BodyVariant, PathParams};
    use std::sync::Arc;

    /// Helper to create a test request
    fn test_request(method: &str, uri: &str) -> Request {
        let builder = http::Request::builder()
            .method(method)
            .uri(uri);
        let req = builder.body(()).unwrap();
        let (parts, _) = req.into_parts();
        Request::new(
            parts,
            BodyVariant::Buffered(Bytes::new()),
            Arc::new(Extensions::new()),
            PathParams::new(),
        )
    }

    /// Helper to create a test request with a header
    fn test_request_with_header(method: &str, uri: &str, key: &str, val: &str) -> Request {
        let builder = http::Request::builder()
            .method(method)
            .uri(uri)
            .header(key, val);
        let req = builder.body(()).unwrap();
        let (parts, _) = req.into_parts();
        Request::new(
            parts,
            BodyVariant::Buffered(Bytes::new()),
            Arc::new(Extensions::new()),
            PathParams::new(),
        )
    }

    /// Simple "next" handler that returns 200 with a text body
    fn ok_next() -> BoxedNext {
        Arc::new(|_req: Request| {
            Box::pin(async move {
                http::Response::builder()
                    .status(200)
                    .body(rustapi_core::ResponseBody::new(Bytes::from("ok")))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        })
    }

    #[test]
    fn test_layer_clone() {
        let runtime = AiRuntime::builder().build();
        let layer = AiContextLayer::new(runtime);
        let _cloned = layer.clone_box();
    }

    #[test]
    fn test_layer_with_budget() {
        let runtime = AiRuntime::builder().build();
        let budget = CostBudget::per_request_usd(1.0);
        let layer = AiContextLayer::new(runtime).with_budget(budget);
        assert!(layer.default_budget.is_some());
    }

    #[tokio::test]
    async fn test_layer_inserts_context() {
        let runtime = AiRuntime::builder().build();
        let layer = AiContextLayer::new(runtime);

        let req = test_request("GET", "/test");

        let next: BoxedNext = Arc::new(|req: Request| {
            Box::pin(async move {
                let has_ctx = req.extensions().get::<RequestContext>().is_some();
                let has_rt = req.extensions().get::<AiRuntime>().is_some();
                let body = format!("ctx={has_ctx},rt={has_rt}");
                http::Response::builder()
                    .status(200)
                    .body(rustapi_core::ResponseBody::new(Bytes::from(body)))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let response = layer.call(req, next).await;
        assert_eq!(response.status().as_u16(), 200);
    }

    #[tokio::test]
    async fn test_layer_picks_up_request_id_header() {
        let runtime = AiRuntime::builder().build();
        let layer = AiContextLayer::new(runtime);

        let req = test_request_with_header("GET", "/test", "x-request-id", "custom-123");

        let next: BoxedNext = Arc::new(|req: Request| {
            Box::pin(async move {
                let ctx = req.extensions().get::<RequestContext>().unwrap();
                let rid = ctx
                    .metadata_value("request_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("none")
                    .to_string();
                http::Response::builder()
                    .status(200)
                    .body(rustapi_core::ResponseBody::new(Bytes::from(rid)))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let response = layer.call(req, next).await;
        assert_eq!(response.status().as_u16(), 200);
    }

    #[tokio::test]
    async fn test_layer_passes_through_to_next() {
        let runtime = AiRuntime::builder().build();
        let layer = AiContextLayer::new(runtime);
        let req = test_request("POST", "/api/chat");

        let response = layer.call(req, ok_next()).await;
        assert_eq!(response.status().as_u16(), 200);
    }
}
