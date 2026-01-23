//! OpenTelemetry middleware layer

use super::config::OtelConfig;
use super::propagation::{extract_trace_context, propagate_trace_context, TraceContext};
use rustapi_core::{
    middleware::{BoxedNext, MiddlewareLayer},
    Request, Response,
};
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

/// OpenTelemetry middleware layer for distributed tracing
#[derive(Clone)]
pub struct OtelLayer {
    config: OtelConfig,
}

impl OtelLayer {
    /// Create a new OtelLayer with the given configuration
    pub fn new(config: OtelConfig) -> Self {
        Self { config }
    }

    /// Create a new OtelLayer with default configuration
    pub fn default_with_service(service_name: impl Into<String>) -> Self {
        Self {
            config: OtelConfig::builder().service_name(service_name).build(),
        }
    }

    /// Check if a path should be excluded from tracing
    fn should_exclude(&self, path: &str) -> bool {
        self.config
            .exclude_paths
            .iter()
            .any(|excluded| path.starts_with(excluded))
    }

    /// Extract header values for tracing
    fn extract_trace_headers(&self, request: &Request) -> Vec<(String, String)> {
        let mut headers = Vec::new();
        for header_name in &self.config.trace_headers {
            if let Some(value) = request.headers().get(header_name.as_str()) {
                if let Ok(val) = value.to_str() {
                    headers.push((header_name.clone(), val.to_string()));
                }
            }
        }
        headers
    }
}

impl MiddlewareLayer for OtelLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();
        let uri = req.uri().to_string();
        let method = req.method().to_string();

        // Check if this path should be excluded
        let path = req.uri().path();
        if self.should_exclude(path) {
            return Box::pin(async move { next(req).await });
        }

        // Extract or create trace context
        let trace_context = extract_trace_context(&req);
        let trace_headers = self.extract_trace_headers(&req);

        Box::pin(async move {
            let start = Instant::now();

            // Create span for this request
            let span_name = format!("{} {}", method, path_pattern(&uri));

            // Log span start
            tracing::info_span!(
                "http_request",
                otel_name = %span_name,
                http_method = %method,
                http_url = %uri,
                http_route = %path_pattern(&uri),
                trace_id = %trace_context.trace_id,
                span_id = %trace_context.span_id,
                parent_span_id = trace_context.parent_span_id.as_deref().unwrap_or("none"),
                service_name = %config.service_name,
            );

            // Store trace context in request extensions for downstream use
            let mut req = req;
            req.extensions_mut().insert(trace_context.clone());

            // Call the next middleware/handler
            let mut response = next(req).await;

            // Calculate duration
            let duration = start.elapsed();
            let status = response.status().as_u16();

            // Determine span status based on HTTP status
            let (span_status, error) = if status >= 500 {
                ("ERROR", true)
            } else if status >= 400 {
                ("UNSET", false)
            } else {
                ("OK", false)
            };

            // Log span end with metrics
            tracing::info!(
                target: "otel",
                trace_id = %trace_context.trace_id,
                span_id = %trace_context.span_id,
                http_method = %method,
                http_url = %uri,
                http_status_code = status,
                duration_ms = duration.as_millis() as u64,
                otel_status = span_status,
                error = error,
                service_name = %config.service_name,
                "request completed"
            );

            // Log trace headers if configured
            for (name, value) in &trace_headers {
                tracing::debug!(
                    target: "otel",
                    trace_id = %trace_context.trace_id,
                    header_name = %name,
                    header_value = %value,
                    "traced header"
                );
            }

            // Propagate trace context to response if enabled
            if config.propagate_context {
                propagate_trace_context(response.headers_mut(), &trace_context);
            }

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

/// Extract a normalized path pattern from the URI
/// Replaces numeric path segments with {id} for better grouping
fn path_pattern(uri: &str) -> String {
    let path = uri.split('?').next().unwrap_or(uri);
    let segments: Vec<&str> = path.split('/').collect();

    segments
        .into_iter()
        .map(|segment| {
            // Replace numeric IDs with {id}
            if segment.chars().all(|c| c.is_ascii_digit()) && !segment.is_empty() {
                "{id}"
            // Replace UUIDs with {uuid}
            } else if is_uuid(segment) {
                "{uuid}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Check if a string looks like a UUID
fn is_uuid(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    parts
        .iter()
        .all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Trait for storing and retrieving trace context from requests
#[allow(dead_code)]
pub trait TraceContextExt {
    /// Get the trace context from the request
    fn trace_context(&self) -> Option<&TraceContext>;
}

impl TraceContextExt for Request {
    fn trace_context(&self) -> Option<&TraceContext> {
        self.extensions().get::<TraceContext>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use rustapi_core::ResponseBody;
    use std::sync::Arc;

    #[test]
    fn test_path_pattern_numeric_ids() {
        assert_eq!(path_pattern("/users/123"), "/users/{id}");
        assert_eq!(
            path_pattern("/users/123/posts/456"),
            "/users/{id}/posts/{id}"
        );
    }

    #[test]
    fn test_path_pattern_uuids() {
        assert_eq!(
            path_pattern("/users/550e8400-e29b-41d4-a716-446655440000"),
            "/users/{uuid}"
        );
    }

    #[test]
    fn test_path_pattern_with_query() {
        assert_eq!(path_pattern("/users/123?page=1"), "/users/{id}");
    }

    #[test]
    fn test_is_uuid() {
        assert!(is_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_uuid("not-a-uuid"));
        assert!(!is_uuid("12345"));
    }

    #[tokio::test]
    async fn test_otel_layer_basic() {
        let config = OtelConfig::builder().service_name("test-service").build();
        let layer = OtelLayer::new(config);

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::new(Bytes::from("OK")))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = http::Request::builder()
            .method("GET")
            .uri("/api/users")
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = layer.call(req, next).await;
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_otel_layer_excludes_health() {
        let config = OtelConfig::builder()
            .service_name("test-service")
            .exclude_path("/health")
            .build();
        let layer = OtelLayer::new(config);

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::new(Bytes::from("OK")))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = http::Request::builder()
            .method("GET")
            .uri("/health")
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = layer.call(req, next).await;
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_trace_context_propagation() {
        let config = OtelConfig::builder()
            .service_name("test-service")
            .propagate_context(true)
            .build();
        let layer = OtelLayer::new(config);

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::new(Bytes::from("OK")))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = http::Request::builder()
            .method("GET")
            .uri("/api/test")
            .header(
                "traceparent",
                "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            )
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = layer.call(req, next).await;
        assert!(response.headers().contains_key("x-trace-id"));
    }
}
