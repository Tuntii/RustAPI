//! ReplayLayer middleware for time-travel debugging.
//!
//! Records HTTP request/response pairs for later replay and diff analysis.
//! Follows the InsightLayer pattern for body capture and response buffering.

use super::memory_store::InMemoryReplayStore;
use super::retention::RetentionJob;
use super::routes;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use rustapi_core::middleware::{BoxedNext, MiddlewareLayer};
use rustapi_core::replay::{
    redact_body, redact_headers, truncate_body, RecordedRequest, RecordedResponse, ReplayConfig,
    ReplayEntry, ReplayMeta, ReplayStore,
};
use rustapi_core::{Request, Response, ResponseBody};
use std::collections::HashMap;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Replay recording middleware layer.
///
/// Captures HTTP request/response pairs and stores them for later replay
/// and diff analysis via the `/__rustapi/replays` admin API.
///
/// # Security
///
/// - Recording disabled by default (`enabled: false`)
/// - Admin token required for all replay endpoints
/// - Sensitive headers redacted automatically
/// - Configurable body field redaction
/// - TTL-based automatic cleanup
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::replay::{ReplayLayer, InMemoryReplayStore};
/// use rustapi_core::replay::ReplayConfig;
///
/// let layer = ReplayLayer::new(
///     ReplayConfig::new()
///         .enabled(true)
///         .admin_token("my-secret-token")
///         .ttl_secs(3600)
/// );
///
/// let app = RustApi::new()
///     .layer(layer)
///     .route("/api/users", get(handler));
/// ```
#[derive(Clone)]
pub struct ReplayLayer {
    config: Arc<ReplayConfig>,
    store: Arc<dyn ReplayStore>,
    retention_started: Arc<AtomicBool>,
}

impl ReplayLayer {
    /// Create a new ReplayLayer with the given configuration.
    ///
    /// Uses an in-memory store with capacity from the config.
    pub fn new(config: ReplayConfig) -> Self {
        let store = InMemoryReplayStore::new(config.store_capacity);
        Self {
            config: Arc::new(config),
            store: Arc::new(store),
            retention_started: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Use a custom store implementation.
    pub fn with_store<S: ReplayStore>(mut self, store: S) -> Self {
        self.store = Arc::new(store);
        self
    }

    /// Get a reference to the replay store.
    pub fn store(&self) -> &Arc<dyn ReplayStore> {
        &self.store
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &ReplayConfig {
        &self.config
    }

    /// Extract client IP from request headers.
    fn extract_client_ip(req: &Request) -> String {
        if let Some(forwarded) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    let ip_str = first_ip.trim();
                    if ip_str.parse::<IpAddr>().is_ok() {
                        return ip_str.to_string();
                    }
                }
            }
        }

        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                let ip_str = ip_str.trim();
                if ip_str.parse::<IpAddr>().is_ok() {
                    return ip_str.to_string();
                }
            }
        }

        "127.0.0.1".to_string()
    }

    /// Extract request ID from headers.
    fn extract_request_id(req: &Request) -> Option<String> {
        for header_name in &["x-request-id", "x-correlation-id", "x-trace-id"] {
            if let Some(value) = req.headers().get(*header_name) {
                if let Ok(id) = value.to_str() {
                    return Some(id.to_string());
                }
            }
        }
        None
    }

    /// Capture all request headers into a HashMap.
    fn capture_headers(headers: &http::HeaderMap) -> HashMap<String, String> {
        let mut captured = HashMap::new();
        for (name, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                captured.insert(name.as_str().to_string(), value_str.to_string());
            }
        }
        captured
    }

    /// Check if body should be captured based on content type.
    fn should_capture_body(headers: &http::HeaderMap, config: &ReplayConfig) -> bool {
        if let Some(content_type) = headers.get(http::header::CONTENT_TYPE) {
            if let Ok(ct) = content_type.to_str() {
                return config.is_capturable_content_type(ct);
            }
        }
        false
    }

    /// Ensure the retention background job is started once.
    fn ensure_retention_started(&self) {
        if !self.retention_started.swap(true, Ordering::SeqCst) {
            let store = self.store.clone();
            let ttl_secs = self.config.ttl_secs;
            let interval = Duration::from_secs(ttl_secs.max(60) / 2);
            RetentionJob::spawn(store, ttl_secs, interval);
        }
    }
}

impl MiddlewareLayer for ReplayLayer {
    fn call(
        &self,
        mut req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();
        let store = self.store.clone();

        // Start retention job on first request
        self.ensure_retention_started();

        Box::pin(async move {
            let path = req.uri().path().to_string();
            let method = req.method().to_string();

            // Handle admin routes: /__rustapi/replays/*
            if path.starts_with(&config.admin_route_prefix) {
                let suffix = &path[config.admin_route_prefix.len()..];
                if let Some(response) = routes::dispatch(
                    req.headers(),
                    &method,
                    req.uri(),
                    store.as_ref(),
                    &config,
                    suffix,
                )
                .await
                {
                    return response;
                }
            }

            // If recording is disabled, pass through
            if !config.enabled {
                return next(req).await;
            }

            // Check path filter
            if !config.should_record_path(&path) {
                return next(req).await;
            }

            // Check sampling
            if !config.should_sample() {
                return next(req).await;
            }

            // Start timing
            let start = Instant::now();

            // Extract request info
            let uri_string = req.uri().to_string();
            let query = req.uri().query().map(|q| q.to_string());
            let client_ip = ReplayLayer::extract_client_ip(&req);
            let request_id = ReplayLayer::extract_request_id(&req);

            // Capture and redact request headers
            let raw_headers = ReplayLayer::capture_headers(req.headers());
            let req_headers = redact_headers(&raw_headers, &config.redact_headers);

            // Capture request body if eligible
            let capture_req_body = ReplayLayer::should_capture_body(req.headers(), &config);

            let (req_body_size, req_body_str, req_body_truncated) = if capture_req_body {
                if let Some(body_bytes) = req.take_body() {
                    let size = body_bytes.len();
                    if size <= config.max_request_body {
                        let body_str = String::from_utf8(body_bytes.to_vec()).ok();
                        // Apply body field redaction
                        let redacted = body_str.and_then(|s| {
                            if config.redact_body_fields.is_empty() {
                                Some(s)
                            } else {
                                redact_body(&s, &config.redact_body_fields, "[REDACTED]")
                            }
                        });
                        (size, redacted, false)
                    } else {
                        // Body too large - truncate
                        let body_str = String::from_utf8(body_bytes.to_vec()).ok();
                        let truncated = body_str.map(|s| {
                            let (t, _) = truncate_body(&s, config.max_request_body);
                            t
                        });
                        (size, truncated, true)
                    }
                } else {
                    (0, None, false)
                }
            } else {
                let size = req
                    .headers()
                    .get(http::header::CONTENT_LENGTH)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                (size, None, false)
            };

            // Call the next handler
            let response = next(req).await;

            // Calculate duration
            let duration = start.elapsed();
            let status = response.status().as_u16();

            // Capture and redact response headers
            let raw_resp_headers = ReplayLayer::capture_headers(response.headers());
            let resp_headers = redact_headers(&raw_resp_headers, &config.redact_headers);

            let capture_resp_body = ReplayLayer::should_capture_body(response.headers(), &config);

            // Buffer response body (must consume and reconstruct)
            let (resp_parts, resp_body) = response.into_parts();
            let resp_body_bytes = match resp_body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => Bytes::new(),
            };

            let resp_body_size = resp_body_bytes.len();
            let (resp_body_str, resp_body_truncated) = if capture_resp_body && resp_body_size > 0 {
                if resp_body_size <= config.max_response_body {
                    let body_str = String::from_utf8(resp_body_bytes.to_vec()).ok();
                    let redacted = body_str.and_then(|s| {
                        if config.redact_body_fields.is_empty() {
                            Some(s)
                        } else {
                            redact_body(&s, &config.redact_body_fields, "[REDACTED]")
                        }
                    });
                    (redacted, false)
                } else {
                    let body_str = String::from_utf8(resp_body_bytes.to_vec()).ok();
                    let truncated = body_str.map(|s| {
                        let (t, _) = truncate_body(&s, config.max_response_body);
                        t
                    });
                    (truncated, true)
                }
            } else {
                (None, false)
            };

            // Build RecordedRequest
            let recorded_request = RecordedRequest {
                method: method.clone(),
                uri: uri_string,
                path: path.clone(),
                query,
                headers: req_headers,
                body: req_body_str,
                body_size: req_body_size,
                body_truncated: req_body_truncated,
            };

            // Build RecordedResponse
            let recorded_response = RecordedResponse {
                status,
                headers: resp_headers,
                body: resp_body_str,
                body_size: resp_body_size,
                body_truncated: resp_body_truncated,
            };

            // Build ReplayMeta
            let mut meta = ReplayMeta::new()
                .with_duration_ms(duration.as_millis() as u64)
                .with_client_ip(client_ip)
                .with_ttl_secs(config.ttl_secs);

            if let Some(req_id) = request_id {
                meta = meta.with_request_id(req_id);
            }

            // Create and store the entry
            let entry = ReplayEntry::new(recorded_request, recorded_response, meta);

            // Store asynchronously (fire and forget, don't block the response)
            let store_clone = store.clone();
            tokio::spawn(async move {
                if let Err(e) = store_clone.store(entry).await {
                    tracing::warn!(error = %e, "Failed to store replay entry");
                }
            });

            // Reconstruct response with the buffered body
            http::Response::from_parts(resp_parts, ResponseBody::Full(Full::new(resp_body_bytes)))
        })
    }

    fn clone_box(&self) -> Box<dyn MiddlewareLayer> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_layer() {
        let layer = ReplayLayer::new(ReplayConfig::new());
        assert!(!layer.config().enabled);
        assert_eq!(layer.config().store_capacity, 500);
    }

    #[test]
    fn test_custom_config() {
        let config = ReplayConfig::new()
            .enabled(true)
            .admin_token("test-token")
            .store_capacity(100)
            .ttl_secs(7200);

        let layer = ReplayLayer::new(config);
        assert!(layer.config().enabled);
        assert_eq!(layer.config().store_capacity, 100);
        assert_eq!(layer.config().ttl_secs, 7200);
    }

    #[test]
    fn test_with_custom_store() {
        let config = ReplayConfig::new().enabled(true);
        let store = InMemoryReplayStore::new(42);

        let layer = ReplayLayer::new(config).with_store(store);
        assert!(layer.config().enabled);
    }
}
