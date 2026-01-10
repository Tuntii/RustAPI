//! Body size limit middleware for RustAPI
//!
//! This module provides middleware to enforce request body size limits,
//! protecting against denial-of-service attacks via large payloads.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_core::middleware::BodyLimitLayer;
//!
//! RustApi::new()
//!     .layer(BodyLimitLayer::new(1024 * 1024)) // 1MB limit
//!     .route("/upload", post(upload_handler))
//!     .run("127.0.0.1:8080")
//!     .await
//! ```

use super::{BoxedNext, MiddlewareLayer};
use crate::error::ApiError;
use crate::request::Request;
use crate::response::{IntoResponse, Response};
use http::StatusCode;
use std::future::Future;
use std::pin::Pin;

/// Default body size limit: 1MB
pub const DEFAULT_BODY_LIMIT: usize = 1024 * 1024;

/// Body size limit middleware layer
///
/// Enforces a maximum size for request bodies. When a request body exceeds
/// the configured limit, a 413 Payload Too Large response is returned.
#[derive(Clone)]
pub struct BodyLimitLayer {
    limit: usize,
}

impl BodyLimitLayer {
    /// Create a new body limit layer with the specified limit in bytes
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum body size in bytes
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // 2MB limit
    /// let layer = BodyLimitLayer::new(2 * 1024 * 1024);
    /// ```
    pub fn new(limit: usize) -> Self {
        Self { limit }
    }

    /// Create a body limit layer with the default limit (1MB)
    pub fn default_limit() -> Self {
        Self::new(DEFAULT_BODY_LIMIT)
    }

    /// Get the configured limit
    pub fn limit(&self) -> usize {
        self.limit
    }
}

impl Default for BodyLimitLayer {
    fn default() -> Self {
        Self::default_limit()
    }
}

impl MiddlewareLayer for BodyLimitLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let limit = self.limit;

        Box::pin(async move {
            // Check Content-Length header first if available
            if let Some(content_length) = req.headers().get(http::header::CONTENT_LENGTH) {
                if let Ok(length_str) = content_length.to_str() {
                    if let Ok(length) = length_str.parse::<usize>() {
                        if length > limit {
                            return ApiError::new(
                                StatusCode::PAYLOAD_TOO_LARGE,
                                "payload_too_large",
                                format!("Request body exceeds limit of {} bytes", limit),
                            )
                            .into_response();
                        }
                    }
                }
            }

            // Also check actual body size (for cases without Content-Length or streaming)
            // The body has already been read at this point in the pipeline
            if let Some(body) = &req.body {
                if body.len() > limit {
                    return ApiError::new(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "payload_too_large",
                        format!("Request body exceeds limit of {} bytes", limit),
                    )
                    .into_response();
                }
            }

            // Body is within limits, continue to next middleware/handler
            next(req).await
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::Request;
    use bytes::Bytes;
    use http::{Extensions, Method};
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Create a test request with the given body
    fn create_test_request_with_body(body: Bytes) -> Request {
        let uri: http::Uri = "/test".parse().unwrap();
        let mut builder = http::Request::builder().method(Method::POST).uri(uri);

        // Set Content-Length header
        builder = builder.header(http::header::CONTENT_LENGTH, body.len().to_string());

        let req = builder.body(()).unwrap();
        let (parts, _) = req.into_parts();

        Request::new(parts, body, Arc::new(Extensions::new()), HashMap::new())
    }

    /// Create a test request without Content-Length header
    fn create_test_request_without_content_length(body: Bytes) -> Request {
        let uri: http::Uri = "/test".parse().unwrap();
        let builder = http::Request::builder().method(Method::POST).uri(uri);

        let req = builder.body(()).unwrap();
        let (parts, _) = req.into_parts();

        Request::new(parts, body, Arc::new(Extensions::new()), HashMap::new())
    }

    /// Create a simple handler that returns 200 OK
    fn ok_handler() -> BoxedNext {
        Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(StatusCode::OK)
                    .body(http_body_util::Full::new(Bytes::from("ok")))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        })
    }

    // **Feature: phase4-ergonomics-v1, Property 3: Body Size Limit Enforcement**
    //
    // For any configured body size limit L and any request body B where size(B) > L,
    // the system should return a 413 Payload Too Large response.
    //
    // **Validates: Requirements 2.2, 2.3, 2.4, 2.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_body_size_limit_enforcement(
            // Generate limit between 1 and 10KB for testing
            limit in 1usize..10240usize,
            // Generate body size relative to limit
            body_size_factor in 0.5f64..2.0f64,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let body_size = ((limit as f64) * body_size_factor) as usize;
                let body = Bytes::from(vec![b'x'; body_size]);
                let request = create_test_request_with_body(body.clone());

                let layer = BodyLimitLayer::new(limit);
                let handler = ok_handler();

                let response = layer.call(request, handler).await;

                if body_size > limit {
                    // Body exceeds limit - should return 413
                    prop_assert_eq!(
                        response.status(),
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "Expected 413 for body size {} > limit {}",
                        body_size,
                        limit
                    );
                } else {
                    // Body within limit - should return 200
                    prop_assert_eq!(
                        response.status(),
                        StatusCode::OK,
                        "Expected 200 for body size {} <= limit {}",
                        body_size,
                        limit
                    );
                }

                Ok(())
            })?;
        }

        #[test]
        fn prop_body_limit_without_content_length_header(
            limit in 1usize..10240usize,
            body_size_factor in 0.5f64..2.0f64,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let body_size = ((limit as f64) * body_size_factor) as usize;
                let body = Bytes::from(vec![b'x'; body_size]);
                // Create request without Content-Length header
                let request = create_test_request_without_content_length(body.clone());

                let layer = BodyLimitLayer::new(limit);
                let handler = ok_handler();

                let response = layer.call(request, handler).await;

                if body_size > limit {
                    // Body exceeds limit - should return 413
                    prop_assert_eq!(
                        response.status(),
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "Expected 413 for body size {} > limit {} (no Content-Length)",
                        body_size,
                        limit
                    );
                } else {
                    // Body within limit - should return 200
                    prop_assert_eq!(
                        response.status(),
                        StatusCode::OK,
                        "Expected 200 for body size {} <= limit {} (no Content-Length)",
                        body_size,
                        limit
                    );
                }

                Ok(())
            })?;
        }
    }

    #[tokio::test]
    async fn test_body_at_exact_limit() {
        let limit = 100;
        let body = Bytes::from(vec![b'x'; limit]);
        let request = create_test_request_with_body(body);

        let layer = BodyLimitLayer::new(limit);
        let handler = ok_handler();

        let response = layer.call(request, handler).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_body_one_byte_over_limit() {
        let limit = 100;
        let body = Bytes::from(vec![b'x'; limit + 1]);
        let request = create_test_request_with_body(body);

        let layer = BodyLimitLayer::new(limit);
        let handler = ok_handler();

        let response = layer.call(request, handler).await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_body_one_byte_under_limit() {
        let limit = 100;
        let body = Bytes::from(vec![b'x'; limit - 1]);
        let request = create_test_request_with_body(body);

        let layer = BodyLimitLayer::new(limit);
        let handler = ok_handler();

        let response = layer.call(request, handler).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_empty_body() {
        let limit = 100;
        let body = Bytes::new();
        let request = create_test_request_with_body(body);

        let layer = BodyLimitLayer::new(limit);
        let handler = ok_handler();

        let response = layer.call(request, handler).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_default_limit() {
        let layer = BodyLimitLayer::default();
        assert_eq!(layer.limit(), DEFAULT_BODY_LIMIT);
    }

    #[test]
    fn test_clone() {
        let layer = BodyLimitLayer::new(1024);
        let cloned = layer.clone();
        assert_eq!(layer.limit(), cloned.limit());
    }
}
