//! Structured request/response logging middleware
//!
//! This module provides detailed logging of HTTP requests and responses
//! with support for correlation IDs, custom fields, and structured output.
//!
//! # Example
//!
//! ```rust,no_run
//! use rustapi_core::RustApi;
//! use rustapi_extras::{LoggingLayer, LogFormat};
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = RustApi::new()
//!         .layer(LoggingLayer::new())
//!         .run("0.0.0.0:3000")
//!         .await
//!         .unwrap();
//! }
//! ```

use rustapi_core::{
    middleware::{BoxedNext, MiddlewareLayer},
    Request, Response,
};
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

/// Logging format
#[derive(Clone, Debug)]
pub enum LogFormat {
    /// Compact format (one line per request)
    Compact,
    /// Detailed format (multi-line with full details)
    Detailed,
    /// JSON format (structured logging)
    Json,
}

/// Logging configuration
#[derive(Clone)]
pub struct LoggingConfig {
    /// Logging format
    pub format: LogFormat,
    /// Whether to log request headers
    pub log_request_headers: bool,
    /// Whether to log response headers
    pub log_response_headers: bool,
    /// Paths to skip logging
    pub skip_paths: Vec<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Compact,
            log_request_headers: false,
            log_response_headers: false,
            skip_paths: vec!["/health".to_string(), "/metrics".to_string()],
        }
    }
}

/// Logging middleware layer
#[derive(Clone)]
pub struct LoggingLayer {
    config: LoggingConfig,
}

impl LoggingLayer {
    /// Create a new logging layer with default configuration
    pub fn new() -> Self {
        Self {
            config: LoggingConfig::default(),
        }
    }

    /// Create a new logging layer with custom configuration
    pub fn with_config(config: LoggingConfig) -> Self {
        Self { config }
    }

    /// Set the logging format
    pub fn format(mut self, format: LogFormat) -> Self {
        self.config.format = format;
        self
    }

    /// Enable request header logging
    pub fn log_request_headers(mut self, enabled: bool) -> Self {
        self.config.log_request_headers = enabled;
        self
    }

    /// Enable response header logging
    pub fn log_response_headers(mut self, enabled: bool) -> Self {
        self.config.log_response_headers = enabled;
        self
    }

    /// Add a path to skip logging
    pub fn skip_path(mut self, path: impl Into<String>) -> Self {
        self.config.skip_paths.push(path.into());
        self
    }
}

impl Default for LoggingLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareLayer for LoggingLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();

        Box::pin(async move {
            let method = req.method().to_string();
            let uri = req.uri().to_string();
            let version = format!("{:?}", req.version());

            // Check if we should skip this path
            if config.skip_paths.iter().any(|p| uri.starts_with(p)) {
                return next(req).await;
            }

            // Get request ID from extensions if available
            let request_id = req
                .extensions()
                .get::<String>()
                .cloned()
                .unwrap_or_else(|| "N/A".to_string());

            let start = Instant::now();

            // Log request
            match config.format {
                LogFormat::Compact => {
                    tracing::info!(
                        request_id = %request_id,
                        method = %method,
                        uri = %uri,
                        version = %version,
                        "incoming request"
                    );
                }
                LogFormat::Detailed => {
                    tracing::info!(
                        request_id = %request_id,
                        method = %method,
                        uri = %uri,
                        version = %version,
                        "=== Incoming Request ==="
                    );

                    if config.log_request_headers {
                        for (name, value) in req.headers() {
                            if let Ok(val) = value.to_str() {
                                tracing::debug!(
                                    request_id = %request_id,
                                    header = %name,
                                    value = %val,
                                    "request header"
                                );
                            }
                        }
                    }
                }
                LogFormat::Json => {
                    let json = serde_json::json!({
                        "type": "request",
                        "request_id": request_id,
                        "method": method,
                        "uri": uri,
                        "version": version,
                    });
                    tracing::info!("{}", json);
                }
            }

            // Call next middleware/handler
            let response = next(req).await;

            let duration = start.elapsed();
            let status = response.status().as_u16();
            let duration_ms = duration.as_millis();

            // Log response
            match config.format {
                LogFormat::Compact => {
                    tracing::info!(
                        request_id = %request_id,
                        method = %method,
                        uri = %uri,
                        status = status,
                        duration_ms = duration_ms,
                        "request completed"
                    );
                }
                LogFormat::Detailed => {
                    tracing::info!(
                        request_id = %request_id,
                        status = status,
                        duration_ms = duration_ms,
                        "=== Response Sent ==="
                    );

                    if config.log_response_headers {
                        for (name, value) in response.headers() {
                            if let Ok(val) = value.to_str() {
                                tracing::debug!(
                                    request_id = %request_id,
                                    header = %name,
                                    value = %val,
                                    "response header"
                                );
                            }
                        }
                    }
                }
                LogFormat::Json => {
                    let json = serde_json::json!({
                        "type": "response",
                        "request_id": request_id,
                        "method": method,
                        "uri": uri,
                        "status": status,
                        "duration_ms": duration_ms,
                    });
                    tracing::info!("{}", json);
                }
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
    use bytes::Bytes;
    use rustapi_core::ResponseBody;
    use std::sync::Arc;

    #[tokio::test]
    async fn logging_middleware_logs_request() {
        let layer = LoggingLayer::new();

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
            .uri("/test")
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = layer.call(req, next).await;
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn logging_middleware_skips_health_check() {
        let layer = LoggingLayer::new();

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
}
