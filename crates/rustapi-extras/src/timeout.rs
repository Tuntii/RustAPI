//! Request timeout middleware
//!
//! This module provides a middleware that enforces timeouts on request handling.
//! If a request takes longer than the specified duration, it will be aborted with a 408 Request Timeout error.
//!
//! # Example
//!
//! ```rust,no_run
//! use rustapi_core::RustApi;
//! use rustapi_extras::TimeoutLayer;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = RustApi::new()
//!         .layer(TimeoutLayer::new(Duration::from_secs(30)))
//!         .run("0.0.0.0:3000")
//!         .await
//!         .unwrap();
//! }
//! ```

use rustapi_core::{middleware::BoxedNext, middleware::MiddlewareLayer, Request, Response, ResponseBody};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Middleware that enforces request timeouts
#[derive(Clone)]
pub struct TimeoutLayer {
    timeout: Duration,
}

impl TimeoutLayer {
    /// Create a new timeout middleware with the given duration
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustapi_extras::TimeoutLayer;
    /// use std::time::Duration;
    ///
    /// let timeout = TimeoutLayer::new(Duration::from_secs(30));
    /// ```
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Create a timeout layer with seconds
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustapi_extras::TimeoutLayer;
    ///
    /// let timeout = TimeoutLayer::from_secs(30);
    /// ```
    pub fn from_secs(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }

    /// Create a timeout layer with milliseconds
    ///
    /// # Example
    ///
    /// ```rust
    /// use rustapi_extras::TimeoutLayer;
    ///
    /// let timeout = TimeoutLayer::from_millis(5000);
    /// ```
    pub fn from_millis(millis: u64) -> Self {
        Self::new(Duration::from_millis(millis))
    }
}

impl MiddlewareLayer for TimeoutLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let timeout = self.timeout;

        Box::pin(async move {
            // Use tokio::time::timeout to enforce the timeout
            match tokio::time::timeout(timeout, next(req)).await {
                Ok(response) => response,
                Err(_) => {
                    // Timeout occurred - return 408 Request Timeout
                    http::Response::builder()
                        .status(408)
                        .header("Content-Type", "application/json")
                        .body(ResponseBody::Full(http_body_util::Full::new(bytes::Bytes::from(
                            serde_json::json!({
                                "error": {
                                    "type": "request_timeout",
                                    "message": format!("Request exceeded timeout of {}ms", timeout.as_millis())
                                }
                            })
                            .to_string(),
                        )))
                        .unwrap()
                }
            }
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use rustapi_core::middleware::MiddlewareLayer;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn timeout_fires_on_slow_request() {
        let timeout_layer = TimeoutLayer::from_millis(100);

        // Create a slow handler that sleeps for 200ms
        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                sleep(Duration::from_millis(200)).await;
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::Full(http_body_util::Full::new(bytes::Bytes::from("OK"))))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = http::Request::builder()
            .method("GET")
            .uri("/")
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = timeout_layer.call(req, next).await;
        assert_eq!(response.status(), 408);
    }

    #[tokio::test]
    async fn timeout_allows_fast_request() {
        let timeout_layer = TimeoutLayer::from_millis(200);

        // Create a fast handler that sleeps for 50ms
        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                sleep(Duration::from_millis(50)).await;
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::Full(http_body_util::Full::new(bytes::Bytes::from("OK"))))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = http::Request::builder()
            .method("GET")
            .uri("/")
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = timeout_layer.call(req, next).await;
        assert_eq!(response.status(), 200);
    }
}
