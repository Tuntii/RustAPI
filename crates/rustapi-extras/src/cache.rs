//! Response Caching Middleware
//!
//! Provides in-memory caching for HTTP responses.
//! Requires `cache` feature.

use bytes::Bytes;
use dashmap::DashMap;
use http_body_util::BodyExt;
use rustapi_core::{
    middleware::{BoxedNext, MiddlewareLayer},
    Request, Response, ResponseBody,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cache configuration
#[derive(Clone)]
pub struct CacheConfig {
    /// Time-to-live for cached items
    pub ttl: Duration,
    /// Methods to cache (e.g., GET, HEAD)
    pub methods: Vec<String>,
    /// Paths to skip caching
    pub skip_paths: Vec<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(60),
            methods: vec!["GET".to_string(), "HEAD".to_string()],
            skip_paths: vec!["/health".to_string()],
        }
    }
}

#[derive(Clone)]
struct CachedResponse {
    status: http::StatusCode,
    headers: http::HeaderMap,
    body: Bytes,
    created_at: Instant,
}

/// In-memory response cache layer
#[derive(Clone)]
pub struct CacheLayer {
    config: CacheConfig,
    store: Arc<DashMap<String, CachedResponse>>,
}

impl CacheLayer {
    /// Create a new cache layer
    pub fn new() -> Self {
        Self {
            config: CacheConfig::default(),
            store: Arc::new(DashMap::new()),
        }
    }

    /// Set TTL
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.config.ttl = ttl;
        self
    }

    /// Add a method to cache
    pub fn add_method(mut self, method: &str) -> Self {
        if !self.config.methods.contains(&method.to_string()) {
            self.config.methods.push(method.to_string());
        }
        self
    }
}

impl Default for CacheLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareLayer for CacheLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();
        let store = self.store.clone();

        Box::pin(async move {
            let method = req.method().to_string();
            let uri = req.uri().to_string();

            // Generate cache key
            let key = format!("{}:{}", method, uri);

            // Check if cachable
            if !config.methods.contains(&method)
                || config.skip_paths.iter().any(|p| uri.starts_with(p))
            {
                return next(req).await;
            }

            // Clean expired entries (simple check on access)
            if let Some(entry) = store.get(&key) {
                if entry.created_at.elapsed() < config.ttl {
                    // Cache hit
                    let mut builder = http::Response::builder().status(entry.status);
                    for (k, v) in &entry.headers {
                        builder = builder.header(k, v);
                    }
                    builder = builder.header("X-Cache", "HIT");

                    return builder
                        .body(ResponseBody::Full(http_body_util::Full::new(entry.body.clone())))
                        .unwrap();
                } else {
                    // Expired
                    drop(entry);
                    store.remove(&key);
                }
            }

            // Cache miss: execute request
            let response = next(req).await;

            // Only cache successful responses
            if response.status().is_success() {
                let (parts, body) = response.into_parts();

                // Buffer the body
                match body.collect().await {
                    Ok(bytes) => {
                        let bytes = bytes.to_bytes();

                        let cached = CachedResponse {
                            status: parts.status,
                            headers: parts.headers.clone(),
                            body: bytes.clone(),
                            created_at: Instant::now(),
                        };

                        store.insert(key, cached);

                        let mut response =
                            http::Response::from_parts(parts, ResponseBody::Full(http_body_util::Full::new(bytes)));
                        response
                            .headers_mut()
                            .insert("X-Cache", "MISS".parse().unwrap());
                        return response;
                    }
                    Err(_) => {
                        return http::Response::builder()
                            .status(500)
                            .body(ResponseBody::Full(http_body_util::Full::new(Bytes::from(
                                "Error buffering response for cache",
                            )))
                            .unwrap();
                    }
                }
            }

            // Return original if buffering failed or not successful
            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}
