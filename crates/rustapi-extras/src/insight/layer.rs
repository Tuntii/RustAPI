//! InsightLayer middleware for traffic data collection.
//!
//! This module provides the main middleware layer that captures
//! request and response information.

use super::config::InsightConfig;
use super::data::InsightData;
use super::store::{InMemoryInsightStore, InsightStore};
use bytes::Bytes;
use http::StatusCode;
use http_body_util::{BodyExt, Full};
use rustapi_core::middleware::{BoxedNext, MiddlewareLayer};
use rustapi_core::{Request, Response};
use serde_json::json;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

/// Traffic insight middleware layer.
///
/// Collects request/response data for analytics, debugging, and monitoring.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::insight::{InsightLayer, InsightConfig};
///
/// let insight = InsightLayer::new()
///     .with_config(InsightConfig::new()
///         .sample_rate(0.5)
///         .skip_path("/health"));
///
/// let app = RustApi::new()
///     .layer(insight)
///     .route("/api", get(handler));
/// ```
#[derive(Clone)]
pub struct InsightLayer {
    config: Arc<InsightConfig>,
    store: Arc<dyn InsightStore>,
}

impl InsightLayer {
    /// Create a new InsightLayer with default configuration.
    pub fn new() -> Self {
        let config = InsightConfig::new();
        let store = InMemoryInsightStore::new(config.store_capacity);
        Self {
            config: Arc::new(config),
            store: Arc::new(store),
        }
    }

    /// Create an InsightLayer with custom configuration.
    pub fn with_config(config: InsightConfig) -> Self {
        let store = InMemoryInsightStore::new(config.store_capacity);
        Self {
            config: Arc::new(config),
            store: Arc::new(store),
        }
    }

    /// Use a custom store implementation.
    pub fn with_store<S: InsightStore>(mut self, store: S) -> Self {
        self.store = Arc::new(store);
        self
    }

    /// Get a reference to the insight store.
    pub fn store(&self) -> &Arc<dyn InsightStore> {
        &self.store
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &InsightConfig {
        &self.config
    }

    /// Extract client IP from request headers.
    fn extract_client_ip(req: &Request) -> String {
        // Try X-Forwarded-For header first
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

        // Try X-Real-IP header
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                let ip_str = ip_str.trim();
                if ip_str.parse::<IpAddr>().is_ok() {
                    return ip_str.to_string();
                }
            }
        }

        // Default to localhost
        "127.0.0.1".to_string()
    }

    /// Extract request ID from headers or generate one.
    fn extract_request_id(req: &Request) -> String {
        // Try common request ID headers
        for header_name in &["x-request-id", "x-correlation-id", "x-trace-id"] {
            if let Some(value) = req.headers().get(*header_name) {
                if let Ok(id) = value.to_str() {
                    return id.to_string();
                }
            }
        }

        // Generate a simple unique ID
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("insight_{:x}", timestamp)
    }

    /// Extract query parameters from URI.
    fn extract_query_params(uri: &http::Uri) -> std::collections::HashMap<String, String> {
        let mut params = std::collections::HashMap::new();
        if let Some(query) = uri.query() {
            for pair in query.split('&') {
                let mut parts = pair.splitn(2, '=');
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    params.insert(
                        urlencoding::decode(key).unwrap_or_default().into_owned(),
                        urlencoding::decode(value).unwrap_or_default().into_owned(),
                    );
                }
            }
        }
        params
    }

    /// Capture headers based on whitelist.
    fn capture_headers(
        headers: &http::HeaderMap,
        config: &InsightConfig,
        is_response: bool,
    ) -> std::collections::HashMap<String, String> {
        let mut captured = std::collections::HashMap::new();

        for (name, value) in headers.iter() {
            let name_str = name.as_str();
            let should_capture = if is_response {
                config.should_capture_response_header(name_str)
            } else {
                config.should_capture_header(name_str)
            };

            if should_capture {
                if let Ok(value_str) = value.to_str() {
                    let final_value = if config.is_sensitive_header(name_str) {
                        "[REDACTED]".to_string()
                    } else {
                        value_str.to_string()
                    };
                    captured.insert(name_str.to_string(), final_value);
                }
            }
        }

        captured
    }

    /// Check if body should be captured based on content type.
    fn should_capture_body(headers: &http::HeaderMap, config: &InsightConfig) -> bool {
        if let Some(content_type) = headers.get(http::header::CONTENT_TYPE) {
            if let Ok(ct) = content_type.to_str() {
                return config.is_capturable_content_type(ct);
            }
        }
        false
    }

    /// Create dashboard response with recent insights.
    fn create_dashboard_response(store: &dyn InsightStore, limit: usize) -> Response {
        let insights = store.get_recent(limit);
        let body = json!({
            "insights": insights,
            "count": insights.len(),
            "total": store.count()
        });

        let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
        http::Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body_bytes)))
            .unwrap()
    }

    /// Create stats response.
    fn create_stats_response(store: &dyn InsightStore) -> Response {
        let stats = store.get_stats();
        let body_bytes = serde_json::to_vec(&stats).unwrap_or_default();
        http::Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body_bytes)))
            .unwrap()
    }
}

impl Default for InsightLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareLayer for InsightLayer {
    fn call(
        &self,
        mut req: Request,
        next: BoxedNext,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'static>> {
        let config = self.config.clone();
        let store = self.store.clone();

        Box::pin(async move {
            let path = req.uri().path().to_string();
            let method = req.method().to_string();

            // Handle dashboard endpoints
            if let Some(ref dashboard_path) = config.dashboard_path {
                if path == *dashboard_path && method == "GET" {
                    // Parse limit from query string
                    let limit = InsightLayer::extract_query_params(req.uri())
                        .get("limit")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(100);
                    return InsightLayer::create_dashboard_response(store.as_ref(), limit);
                }
            }

            if let Some(ref stats_path) = config.stats_path {
                if path == *stats_path && method == "GET" {
                    return InsightLayer::create_stats_response(store.as_ref());
                }
            }

            // Check if this path should be skipped
            if config.should_skip_path(&path) {
                return next(req).await;
            }

            // Check sampling
            if !config.should_sample() {
                return next(req).await;
            }

            // Start timing
            let start = Instant::now();

            // Extract request info before calling next
            let request_id = InsightLayer::extract_request_id(&req);
            let client_ip = InsightLayer::extract_client_ip(&req);
            let query_params = InsightLayer::extract_query_params(req.uri());
            let request_headers = InsightLayer::capture_headers(req.headers(), &config, false);
            let capture_request_body =
                config.capture_request_body && InsightLayer::should_capture_body(req.headers(), &config);

            // Get request body info if body capture is enabled
            // Note: take_body() consumes the body, so we can only capture OR process, not both
            // For insight purposes, we estimate size from content-length header when not capturing
            let (request_size, request_body_capture) = if capture_request_body {
                if let Some(body_bytes) = req.take_body() {
                    let size = body_bytes.len();
                    let body_str = if size <= config.max_body_size {
                        String::from_utf8(body_bytes.to_vec()).ok()
                    } else {
                        None
                    };
                    (size, body_str)
                } else {
                    (0, None)
                }
            } else {
                // Estimate size from Content-Length header
                let size = req.headers()
                    .get(http::header::CONTENT_LENGTH)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                (size, None)
            };

            // Call the next handler
            let response = next(req).await;

            // Calculate duration
            let duration = start.elapsed();
            let status = response.status().as_u16();

            // Capture response info
            let response_headers =
                InsightLayer::capture_headers(response.headers(), &config, true);
            let capture_response_body =
                config.capture_response_body && InsightLayer::should_capture_body(response.headers(), &config);

            // Buffer response body if needed
            let (resp_parts, resp_body) = response.into_parts();
            let resp_body_bytes = match resp_body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => Bytes::new(),
            };

            let response_size = resp_body_bytes.len();
            let response_body_capture =
                if capture_response_body && response_size <= config.max_body_size {
                    String::from_utf8(resp_body_bytes.to_vec()).ok()
                } else {
                    None
                };

            // Create insight
            let mut insight = InsightData::new(&request_id, &method, &path)
                .with_status(status)
                .with_duration(duration)
                .with_client_ip(&client_ip)
                .with_request_size(request_size)
                .with_response_size(response_size);

            // Add query params
            for (key, value) in query_params {
                insight.add_query_param(key, value);
            }

            // Add headers
            for (key, value) in request_headers {
                insight.add_request_header(key, value);
            }
            for (key, value) in response_headers {
                insight.add_response_header(key, value);
            }

            // Add body captures
            if let Some(body) = request_body_capture {
                insight.set_request_body(body);
            }
            if let Some(body) = response_body_capture {
                insight.set_response_body(body);
            }

            // Invoke callback if configured
            if let Some(ref callback) = config.on_insight {
                callback(&insight);
            }

            // Store the insight
            store.store(insight);

            // Reconstruct response
            http::Response::from_parts(resp_parts, Full::new(resp_body_bytes))
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
    fn test_extract_query_params() {
        let uri: http::Uri = "/users?page=1&limit=10".parse().unwrap();
        let params = InsightLayer::extract_query_params(&uri);

        assert_eq!(params.get("page"), Some(&"1".to_string()));
        assert_eq!(params.get("limit"), Some(&"10".to_string()));
    }

    #[test]
    fn test_capture_headers_with_whitelist() {
        let mut headers = http::HeaderMap::new();
        headers.insert(http::header::CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(http::header::USER_AGENT, "test-agent".parse().unwrap());
        headers.insert(http::header::AUTHORIZATION, "Bearer secret".parse().unwrap());

        let config = InsightConfig::new()
            .header_whitelist(vec!["content-type", "authorization"]);

        let captured = InsightLayer::capture_headers(&headers, &config, false);

        assert_eq!(captured.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(captured.get("authorization"), Some(&"[REDACTED]".to_string()));
        assert!(!captured.contains_key("user-agent"));
    }

    #[test]
    fn test_default_layer() {
        let layer = InsightLayer::new();
        assert_eq!(layer.config().sample_rate, 1.0);
        assert_eq!(layer.config().store_capacity, 1000);
    }

    #[test]
    fn test_custom_config() {
        let config = InsightConfig::new()
            .sample_rate(0.5)
            .max_body_size(8192)
            .skip_path("/health");

        let layer = InsightLayer::with_config(config);

        assert_eq!(layer.config().sample_rate, 0.5);
        assert_eq!(layer.config().max_body_size, 8192);
    }
}
