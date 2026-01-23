//! API Key authentication middleware
//!
//! This module provides API key-based authentication for securing endpoints.
//! Supports both header-based and query parameter API keys.
//!
//! # Example
//!
//! ```rust,no_run
//! use rustapi_core::RustApi;
//! use rustapi_extras::ApiKeyLayer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = RustApi::new()
//!         .layer(
//!             ApiKeyLayer::new()
//!                 .header("X-API-Key")
//!                 .add_key("your-secret-api-key-here")
//!         )
//!         .run("0.0.0.0:3000")
//!         .await
//!         .unwrap();
//! }
//! ```

use rustapi_core::{
    middleware::{BoxedNext, MiddlewareLayer},
    Request, Response, ResponseBody,
};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// API Key authentication configuration
#[derive(Clone)]
pub struct ApiKeyConfig {
    /// Valid API keys
    pub keys: Arc<HashSet<String>>,
    /// Header name to check for API key
    pub header_name: String,
    /// Query parameter name to check for API key
    pub query_param_name: Option<String>,
    /// Paths to skip API key validation
    pub skip_paths: Vec<String>,
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            keys: Arc::new(HashSet::new()),
            header_name: "X-API-Key".to_string(),
            query_param_name: None,
            skip_paths: vec!["/health".to_string(), "/docs".to_string()],
        }
    }
}

/// API Key authentication middleware
#[derive(Clone)]
pub struct ApiKeyLayer {
    config: ApiKeyConfig,
}

impl ApiKeyLayer {
    /// Create a new API key layer with default configuration
    pub fn new() -> Self {
        Self {
            config: ApiKeyConfig::default(),
        }
    }

    /// Set the header name to check for API key
    pub fn header(mut self, name: impl Into<String>) -> Self {
        self.config.header_name = name.into();
        self
    }

    /// Enable query parameter API key checking
    pub fn query_param(mut self, name: impl Into<String>) -> Self {
        self.config.query_param_name = Some(name.into());
        self
    }

    /// Add a valid API key
    pub fn add_key(mut self, key: impl Into<String>) -> Self {
        let keys = Arc::make_mut(&mut self.config.keys);
        keys.insert(key.into());
        self
    }

    /// Add multiple valid API keys
    pub fn add_keys(mut self, keys: Vec<String>) -> Self {
        let key_set = Arc::make_mut(&mut self.config.keys);
        for key in keys {
            key_set.insert(key);
        }
        self
    }

    /// Skip API key validation for specific paths
    pub fn skip_path(mut self, path: impl Into<String>) -> Self {
        self.config.skip_paths.push(path.into());
        self
    }
}

impl Default for ApiKeyLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareLayer for ApiKeyLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();

        Box::pin(async move {
            let path = req.uri().path();

            // Check if this path should skip validation
            if config.skip_paths.iter().any(|p| path.starts_with(p)) {
                return next(req).await;
            }

            // Try to extract API key from header
            let api_key = if let Some(header_value) = req.headers().get(&config.header_name) {
                header_value.to_str().ok()
            } else {
                None
            };

            // If not in header, try query parameter
            let api_key = if api_key.is_none() {
                if let Some(query_param) = &config.query_param_name {
                    req.uri().query().and_then(|q| {
                        q.split('&').find_map(|param| {
                            let mut parts = param.split('=');
                            if parts.next()? == query_param {
                                parts.next()
                            } else {
                                None
                            }
                        })
                    })
                } else {
                    None
                }
            } else {
                api_key
            };

            // Validate API key
            match api_key {
                Some(key) if config.keys.contains(key) => {
                    // Valid API key, proceed
                    next(req).await
                }
                Some(_) => {
                    // Invalid API key
                    create_unauthorized_response("Invalid API key")
                }
                None => {
                    // Missing API key
                    create_unauthorized_response("Missing API key")
                }
            }
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

fn create_unauthorized_response(message: &str) -> Response {
    let error_body = serde_json::json!({
        "error": {
            "type": "unauthorized",
            "message": message
        }
    });

    let body = serde_json::to_vec(&error_body).unwrap_or_default();

    http::Response::builder()
        .status(401)
        .header("Content-Type", "application/json")
        .body(ResponseBody::Full(http_body_util::Full::new(
            bytes::Bytes::from(body),
        )))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::sync::Arc;

    #[tokio::test]
    async fn api_key_valid_header() {
        let layer = ApiKeyLayer::new()
            .header("X-API-Key")
            .add_key("test-key-123");

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::Full(http_body_util::Full::new(
                        bytes::Bytes::from("OK"),
                    )))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = http::Request::builder()
            .method("GET")
            .uri("/api/users")
            .header("X-API-Key", "test-key-123")
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = layer.call(req, next).await;
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn api_key_invalid_header() {
        let layer = ApiKeyLayer::new()
            .header("X-API-Key")
            .add_key("test-key-123");

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::Full(http_body_util::Full::new(
                        bytes::Bytes::from("OK"),
                    )))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = http::Request::builder()
            .method("GET")
            .uri("/api/users")
            .header("X-API-Key", "wrong-key")
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = layer.call(req, next).await;
        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn api_key_missing() {
        let layer = ApiKeyLayer::new()
            .header("X-API-Key")
            .add_key("test-key-123");

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::Full(http_body_util::Full::new(
                        bytes::Bytes::from("OK"),
                    )))
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
        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn api_key_skips_health_check() {
        let layer = ApiKeyLayer::new()
            .header("X-API-Key")
            .add_key("test-key-123");

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::Full(http_body_util::Full::new(
                        bytes::Bytes::from("OK"),
                    )))
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
    async fn api_key_query_param() {
        let layer = ApiKeyLayer::new()
            .query_param("api_key")
            .add_key("test-key-123");

        let next: BoxedNext = Arc::new(|_req: Request| {
            Box::pin(async {
                http::Response::builder()
                    .status(200)
                    .body(ResponseBody::Full(http_body_util::Full::new(
                        bytes::Bytes::from("OK"),
                    )))
                    .unwrap()
            }) as Pin<Box<dyn Future<Output = Response> + Send + 'static>>
        });

        let req = http::Request::builder()
            .method("GET")
            .uri("/api/users?api_key=test-key-123")
            .body(())
            .unwrap();
        let req = Request::from_http_request(req, Bytes::new());

        let response = layer.call(req, next).await;
        assert_eq!(response.status(), 200);
    }
}
