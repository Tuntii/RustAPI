//! Request Deduplication Middleware
//!
//! Prevents processing of duplicate requests based on an Idempotency-Key header.
//! Requires `dedup` feature.

use dashmap::DashMap;
use rustapi_core::{
    middleware::{BoxedNext, MiddlewareLayer},
    Request, Response, ResponseBody,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Deduplication configuration
#[derive(Clone)]
pub struct DedupConfig {
    /// Name of the header containing the idempotency key
    pub header_name: String,
    /// Time-to-live for deduplication entries
    pub ttl: Duration,
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            header_name: "Idempotency-Key".to_string(),
            ttl: Duration::from_secs(300), // 5 minutes default
        }
    }
}

/// Deduplication middleware layer
#[derive(Clone)]
pub struct DedupLayer {
    config: DedupConfig,
    /// Stores idempotency keys and their creation time.
    /// Value is optional Response if we wanted to support caching (not implemented in V1)
    /// For now, just tracks presence.
    store: Arc<DashMap<String, Instant>>,
}

impl DedupLayer {
    /// Create a new deduplication layer
    pub fn new() -> Self {
        Self {
            config: DedupConfig::default(),
            store: Arc::new(DashMap::new()),
        }
    }

    /// Set custom header name
    pub fn header_name(mut self, name: impl Into<String>) -> Self {
        self.config.header_name = name.into();
        self
    }

    /// Set TTL
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.config.ttl = ttl;
        self
    }
}

impl Default for DedupLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareLayer for DedupLayer {
    fn call(
        &self,
        req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();
        let store = self.store.clone();

        Box::pin(async move {
            // Check for idempotency key
            let key = if let Some(val) = req.headers().get(&config.header_name) {
                match val.to_str() {
                    Ok(s) => s.to_string(),
                    Err(_) => return next(req).await, // Invalid header value, proceed as normal? or Error? Proceeding is safer.
                }
            } else {
                // No key, proceed normally
                return next(req).await;
            };

            // Check if key exists and is valid
            if let Some(created_at) = store.get(&key) {
                if created_at.elapsed() < config.ttl {
                    // Duplicate request detected
                    // Determine if processing or finished. For V1 we just say "Conflict / Already Processed"
                    return http::Response::builder()
                        .status(409) // Conflict
                        .header("Content-Type", "application/json")
                        .body(ResponseBody::Full(http_body_util::Full::new(bytes::Bytes::from(
                            serde_json::json!({
                                "error": {
                                    "type": "duplicate_request",
                                    "message": format!("Request with key '{}' has already been processed or is processing", key)
                                }
                            })
                            .to_string(),
                        )))
                        .unwrap();
                } else {
                    // Expired, remove
                    drop(created_at);
                    store.remove(&key);
                }
            }

            // New key, track it
            store.insert(key.clone(), Instant::now());

            // Process request
            // Note: In a robust implementation, we might want to remove the key if processing fails,
            // or update it with the response for caching (Idempotency Cache pattern).
            // For simple Deduplication (prevent double-submit), keeping it is fine.
            let response = next(req).await;

            response
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}
