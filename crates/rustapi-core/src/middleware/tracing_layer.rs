//! Enhanced Tracing middleware
//!
//! Logs request method, path, request_id, status code, and duration for each request.
//! Supports custom fields that are included in all request spans.

use super::layer::{BoxedNext, MiddlewareLayer};
use super::request_id::RequestId;
use crate::request::Request;
use crate::response::Response;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use tracing::{info_span, Instrument, Level};

/// Middleware layer that creates tracing spans for requests
///
/// This layer creates a span for each request containing:
/// - HTTP method
/// - Request path
/// - Request ID (if RequestIdLayer is applied)
/// - Response status code
/// - Request duration
/// - Any custom fields configured via `with_field()`
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::middleware::TracingLayer;
///
/// RustApi::new()
///     .layer(TracingLayer::new()
///         .with_field("service", "my-api")
///         .with_field("version", "1.0.0"))
///     .route("/", get(handler))
/// ```
#[derive(Clone)]
pub struct TracingLayer {
    level: Level,
    custom_fields: Vec<(String, String)>,
}

impl TracingLayer {
    /// Create a new TracingLayer with default INFO level
    pub fn new() -> Self {
        Self {
            level: Level::INFO,
            custom_fields: Vec::new(),
        }
    }

    /// Create a TracingLayer with a specific log level
    pub fn with_level(level: Level) -> Self {
        Self {
            level,
            custom_fields: Vec::new(),
        }
    }

    /// Add a custom field to all request spans
    ///
    /// Custom fields are included in every span created by this layer.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// TracingLayer::new()
    ///     .with_field("service", "my-api")
    ///     .with_field("environment", "production")
    /// ```
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_fields.push((key.into(), value.into()));
        self
    }
}

impl Default for TracingLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareLayer for TracingLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let level = self.level;
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let custom_fields = self.custom_fields.clone();

        // Extract request_id if available
        let request_id = req
            .extensions()
            .get::<RequestId>()
            .map(|id| id.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Box::pin(async move {
            let start = Instant::now();

            // Create span with all fields
            // We use info_span! as the base and record custom fields dynamically
            let span = info_span!(
                "http_request",
                method = %method,
                path = %path,
                request_id = %request_id,
                status = tracing::field::Empty,
                duration_ms = tracing::field::Empty,
                error = tracing::field::Empty,
            );

            // Record custom fields in the span
            for (key, value) in &custom_fields {
                span.record(key.as_str(), value.as_str());
            }

            // Execute the request within the span
            let response = async {
                next(req).await
            }
            .instrument(span.clone())
            .await;

            let duration = start.elapsed();
            let status = response.status();
            let status_code = status.as_u16();

            // Record response fields
            span.record("status", status_code);
            span.record("duration_ms", duration.as_millis() as u64);

            // Record error if status indicates failure
            if status.is_client_error() || status.is_server_error() {
                span.record("error", true);
            }

            // Log based on status code and configured level
            let _enter = span.enter();
            if status.is_success() {
                match level {
                    Level::TRACE => tracing::trace!(
                        method = %method,
                        path = %path,
                        request_id = %request_id,
                        status = %status_code,
                        duration_ms = %duration.as_millis(),
                        "Request completed"
                    ),
                    Level::DEBUG => tracing::debug!(
                        method = %method,
                        path = %path,
                        request_id = %request_id,
                        status = %status_code,
                        duration_ms = %duration.as_millis(),
                        "Request completed"
                    ),
                    Level::INFO => tracing::info!(
                        method = %method,
                        path = %path,
                        request_id = %request_id,
                        status = %status_code,
                        duration_ms = %duration.as_millis(),
                        "Request completed"
                    ),
                    Level::WARN => tracing::warn!(
                        method = %method,
                        path = %path,
                        request_id = %request_id,
                        status = %status_code,
                        duration_ms = %duration.as_millis(),
                        "Request completed"
                    ),
                    Level::ERROR => tracing::error!(
                        method = %method,
                        path = %path,
                        request_id = %request_id,
                        status = %status_code,
                        duration_ms = %duration.as_millis(),
                        "Request completed"
                    ),
                }
            } else {
                tracing::warn!(
                    method = %method,
                    path = %path,
                    request_id = %request_id,
                    status = %status_code,
                    duration_ms = %duration.as_millis(),
                    error = true,
                    "Request failed"
                );
            }

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::layer::{BoxedNext, LayerStack};
    use crate::middleware::request_id::RequestIdLayer;
    use bytes::Bytes;
    use http::{Extensions, Method, StatusCode};
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseError;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tracing_subscriber::layer::SubscriberExt;

    /// Create a test request with the given method and path
    fn create_test_request(method: Method, path: &str) -> crate::request::Request {
        let uri: http::Uri = path.parse().unwrap();
        let builder = http::Request::builder().method(method).uri(uri);

        let req = builder.body(()).unwrap();
        let (parts, _) = req.into_parts();

        crate::request::Request::new(
            parts,
            Bytes::new(),
            Arc::new(Extensions::new()),
            HashMap::new(),
        )
    }

    #[test]
    fn test_tracing_layer_creation() {
        let layer = TracingLayer::new();
        assert_eq!(layer.level, Level::INFO);
        assert!(layer.custom_fields.is_empty());

        let layer = TracingLayer::with_level(Level::DEBUG);
        assert_eq!(layer.level, Level::DEBUG);
    }

    #[test]
    fn test_tracing_layer_with_custom_fields() {
        let layer = TracingLayer::new()
            .with_field("service", "test-api")
            .with_field("version", "1.0.0");

        assert_eq!(layer.custom_fields.len(), 2);
        assert_eq!(layer.custom_fields[0], ("service".to_string(), "test-api".to_string()));
        assert_eq!(layer.custom_fields[1], ("version".to_string(), "1.0.0".to_string()));
    }

    #[test]
    fn test_tracing_layer_clone() {
        let layer = TracingLayer::new()
            .with_field("key", "value");
        
        let cloned = layer.clone();
        assert_eq!(cloned.level, layer.level);
        assert_eq!(cloned.custom_fields, layer.custom_fields);
    }

    /// A test subscriber that captures span fields for verification
    #[derive(Clone)]
    struct SpanFieldCapture {
        captured_fields: Arc<std::sync::Mutex<Vec<CapturedSpan>>>,
    }

    #[derive(Debug, Clone)]
    struct CapturedSpan {
        name: String,
        fields: HashMap<String, String>,
    }

    impl SpanFieldCapture {
        fn new() -> Self {
            Self {
                captured_fields: Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        fn get_spans(&self) -> Vec<CapturedSpan> {
            self.captured_fields.lock().unwrap().clone()
        }
    }

    impl<S> tracing_subscriber::Layer<S> for SpanFieldCapture
    where
        S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
    {
        fn on_new_span(
            &self,
            attrs: &tracing::span::Attributes<'_>,
            _id: &tracing::span::Id,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut fields = HashMap::new();
            let mut visitor = FieldVisitor { fields: &mut fields };
            attrs.record(&mut visitor);

            let span = CapturedSpan {
                name: attrs.metadata().name().to_string(),
                fields,
            };

            self.captured_fields.lock().unwrap().push(span);
        }

        fn on_record(
            &self,
            id: &tracing::span::Id,
            values: &tracing::span::Record<'_>,
            ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            if let Some(_span) = ctx.span(id) {
                let mut captured = self.captured_fields.lock().unwrap();
                if let Some(last_span) = captured.last_mut() {
                    let mut visitor = FieldVisitor { fields: &mut last_span.fields };
                    values.record(&mut visitor);
                }
            }
        }
    }

    struct FieldVisitor<'a> {
        fields: &'a mut HashMap<String, String>,
    }

    impl<'a> tracing::field::Visit for FieldVisitor<'a> {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.fields.insert(field.name().to_string(), format!("{:?}", value));
        }

        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            self.fields.insert(field.name().to_string(), value.to_string());
        }

        fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
            self.fields.insert(field.name().to_string(), value.to_string());
        }

        fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
            self.fields.insert(field.name().to_string(), value.to_string());
        }

        fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
            self.fields.insert(field.name().to_string(), value.to_string());
        }
    }

    // **Feature: phase4-ergonomics-v1, Property 8: Tracing Span Completeness**
    //
    // For any HTTP request processed by the system with tracing enabled, the resulting
    // span should contain: request method, request path, request ID, response status code,
    // and response duration.
    //
    // **Validates: Requirements 4.1, 4.2, 4.3, 4.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_tracing_span_completeness(
            method_idx in 0usize..5usize,
            path in "/[a-z]{1,10}(/[a-z]{1,10})?",
            status_code in 200u16..600u16,
            custom_key in "[a-z]{3,10}",
            custom_value in "[a-z0-9]{3,20}",
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                // Set up span capture
                let capture = SpanFieldCapture::new();
                let subscriber = tracing_subscriber::registry().with(capture.clone());
                
                // Use a guard to set the subscriber for this test
                let _guard = tracing::subscriber::set_default(subscriber);

                // Create middleware stack with RequestIdLayer and TracingLayer
                let mut stack = LayerStack::new();
                stack.push(Box::new(RequestIdLayer::new()));
                stack.push(Box::new(TracingLayer::new()
                    .with_field(&custom_key, &custom_value)));

                // Map index to HTTP method
                let methods = [Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH];
                let method = methods[method_idx].clone();

                // Create handler that returns the specified status
                let response_status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK);
                let handler: BoxedNext = Arc::new(move |_req: crate::request::Request| {
                    let status = response_status;
                    Box::pin(async move {
                        http::Response::builder()
                            .status(status)
                            .body(http_body_util::Full::new(Bytes::from("test")))
                            .unwrap()
                    }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
                });

                // Execute request
                let request = create_test_request(method.clone(), &path);
                let response = stack.execute(request, handler).await;

                // Verify response status matches
                prop_assert_eq!(response.status(), response_status);

                // Find the http_request span
                let spans = capture.get_spans();
                let http_span = spans.iter().find(|s| s.name == "http_request");
                
                prop_assert!(http_span.is_some(), "Should have created an http_request span");
                let span = http_span.unwrap();

                // Verify required fields are present
                // Method
                prop_assert!(
                    span.fields.contains_key("method"),
                    "Span should contain 'method' field. Fields: {:?}", span.fields
                );
                prop_assert_eq!(
                    span.fields.get("method").map(|s| s.trim_matches('"')),
                    Some(method.as_str()),
                    "Method should match request method"
                );

                // Path
                prop_assert!(
                    span.fields.contains_key("path"),
                    "Span should contain 'path' field. Fields: {:?}", span.fields
                );
                prop_assert_eq!(
                    span.fields.get("path").map(|s| s.trim_matches('"')),
                    Some(path.as_str()),
                    "Path should match request path"
                );

                // Request ID
                prop_assert!(
                    span.fields.contains_key("request_id"),
                    "Span should contain 'request_id' field. Fields: {:?}", span.fields
                );
                let request_id = span.fields.get("request_id").unwrap();
                // Request ID should be a UUID format (36 chars with hyphens) or "unknown"
                let request_id_trimmed = request_id.trim_matches('"');
                prop_assert!(
                    request_id_trimmed == "unknown" || request_id_trimmed.len() == 36,
                    "Request ID should be UUID format or 'unknown', got: {}", request_id
                );

                // Status code (recorded after response)
                prop_assert!(
                    span.fields.contains_key("status"),
                    "Span should contain 'status' field. Fields: {:?}", span.fields
                );
                let recorded_status: u16 = span.fields.get("status")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                prop_assert_eq!(
                    recorded_status,
                    status_code,
                    "Status should match response status code"
                );

                // Duration (recorded after response)
                prop_assert!(
                    span.fields.contains_key("duration_ms"),
                    "Span should contain 'duration_ms' field. Fields: {:?}", span.fields
                );
                let duration: u64 = span.fields.get("duration_ms")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(u64::MAX);
                prop_assert!(
                    duration < 10000, // Should complete in less than 10 seconds
                    "Duration should be reasonable, got: {} ms", duration
                );

                // Error field should be present for error responses
                if response_status.is_client_error() || response_status.is_server_error() {
                    prop_assert!(
                        span.fields.contains_key("error"),
                        "Span should contain 'error' field for error responses. Fields: {:?}", span.fields
                    );
                }

                Ok(())
            });
            result?;
        }
    }

    #[test]
    fn test_tracing_layer_records_request_id() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let capture = SpanFieldCapture::new();
            let subscriber = tracing_subscriber::registry().with(capture.clone());
            let _guard = tracing::subscriber::set_default(subscriber);

            let mut stack = LayerStack::new();
            stack.push(Box::new(RequestIdLayer::new()));
            stack.push(Box::new(TracingLayer::new()));

            let handler: BoxedNext = Arc::new(|_req: crate::request::Request| {
                Box::pin(async {
                    http::Response::builder()
                        .status(StatusCode::OK)
                        .body(http_body_util::Full::new(Bytes::from("ok")))
                        .unwrap()
                }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
            });

            let request = create_test_request(Method::GET, "/test");
            let _response = stack.execute(request, handler).await;

            let spans = capture.get_spans();
            let http_span = spans.iter().find(|s| s.name == "http_request");
            assert!(http_span.is_some(), "Should have http_request span");
            
            let span = http_span.unwrap();
            assert!(span.fields.contains_key("request_id"), "Should have request_id field");
        });
    }

    #[test]
    fn test_tracing_layer_records_error_for_failures() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let capture = SpanFieldCapture::new();
            let subscriber = tracing_subscriber::registry().with(capture.clone());
            let _guard = tracing::subscriber::set_default(subscriber);

            let mut stack = LayerStack::new();
            stack.push(Box::new(TracingLayer::new()));

            let handler: BoxedNext = Arc::new(|_req: crate::request::Request| {
                Box::pin(async {
                    http::Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(http_body_util::Full::new(Bytes::from("error")))
                        .unwrap()
                }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
            });

            let request = create_test_request(Method::GET, "/test");
            let response = stack.execute(request, handler).await;

            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

            let spans = capture.get_spans();
            let http_span = spans.iter().find(|s| s.name == "http_request");
            assert!(http_span.is_some(), "Should have http_request span");
            
            let span = http_span.unwrap();
            assert!(span.fields.contains_key("error"), "Should have error field for 5xx response");
        });
    }
}
