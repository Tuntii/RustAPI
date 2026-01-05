//! Configuration for the InsightLayer middleware.
//!
//! This module provides the `InsightConfig` builder for customizing
//! traffic insight collection behavior.

use super::data::InsightData;
use std::collections::HashSet;
use std::sync::Arc;

/// Callback function type for processing insights.
pub type InsightCallback = Arc<dyn Fn(&InsightData) + Send + Sync>;

/// Configuration for the InsightLayer middleware.
///
/// Use the builder pattern to customize behavior:
///
/// ```ignore
/// use rustapi_extras::insight::InsightConfig;
///
/// let config = InsightConfig::new()
///     .sample_rate(0.5)           // Sample 50% of requests
///     .max_body_size(4096)        // Capture up to 4KB of body
///     .skip_path("/health")       // Exclude health checks
///     .capture_request_body(true) // Enable request body capture
///     .header_whitelist(vec!["content-type", "user-agent"]);
/// ```
#[derive(Clone)]
pub struct InsightConfig {
    /// Sampling rate (0.0-1.0). 1.0 = all requests, 0.5 = 50% of requests.
    pub(crate) sample_rate: f64,

    /// Maximum body size to capture (in bytes). Default: 4096 (4KB).
    pub(crate) max_body_size: usize,

    /// Paths to skip from insight collection.
    pub(crate) skip_paths: HashSet<String>,

    /// Path prefixes to skip from insight collection.
    pub(crate) skip_path_prefixes: HashSet<String>,

    /// Request headers to capture (empty = none, use `*` for all).
    pub(crate) header_whitelist: HashSet<String>,

    /// Response headers to capture (empty = none, use `*` for all).
    pub(crate) response_header_whitelist: HashSet<String>,

    /// Whether to capture request bodies. Default: false.
    pub(crate) capture_request_body: bool,

    /// Whether to capture response bodies. Default: false.
    pub(crate) capture_response_body: bool,

    /// Callback to invoke for each insight (optional).
    pub(crate) on_insight: Option<InsightCallback>,

    /// Dashboard endpoint path. Set to None to disable. Default: "/insights".
    pub(crate) dashboard_path: Option<String>,

    /// Stats endpoint path. Set to None to disable. Default: "/insights/stats".
    pub(crate) stats_path: Option<String>,

    /// Storage capacity for in-memory store. Default: 1000.
    pub(crate) store_capacity: usize,

    /// Sensitive headers to redact (values replaced with "[REDACTED]").
    pub(crate) sensitive_headers: HashSet<String>,

    /// Content types to capture body for. Default: application/json, text/*.
    pub(crate) capturable_content_types: HashSet<String>,
}

impl Default for InsightConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl InsightConfig {
    /// Create a new configuration with default values.
    ///
    /// Defaults:
    /// - Sample rate: 1.0 (all requests)
    /// - Max body size: 4096 bytes (4KB)
    /// - No paths skipped
    /// - No headers captured
    /// - Body capture disabled
    /// - Dashboard at "/insights"
    /// - Stats at "/insights/stats"
    /// - Store capacity: 1000 entries
    pub fn new() -> Self {
        let mut sensitive = HashSet::new();
        sensitive.insert("authorization".to_string());
        sensitive.insert("cookie".to_string());
        sensitive.insert("x-api-key".to_string());
        sensitive.insert("x-auth-token".to_string());

        let mut capturable = HashSet::new();
        capturable.insert("application/json".to_string());
        capturable.insert("text/plain".to_string());
        capturable.insert("text/html".to_string());
        capturable.insert("application/xml".to_string());
        capturable.insert("text/xml".to_string());

        Self {
            sample_rate: 1.0,
            max_body_size: 4096,
            skip_paths: HashSet::new(),
            skip_path_prefixes: HashSet::new(),
            header_whitelist: HashSet::new(),
            response_header_whitelist: HashSet::new(),
            capture_request_body: false,
            capture_response_body: false,
            on_insight: None,
            dashboard_path: Some("/insights".to_string()),
            stats_path: Some("/insights/stats".to_string()),
            store_capacity: 1000,
            sensitive_headers: sensitive,
            capturable_content_types: capturable,
        }
    }

    /// Set the sampling rate (0.0 to 1.0).
    ///
    /// # Arguments
    ///
    /// * `rate` - Fraction of requests to sample. 1.0 = all, 0.1 = 10%.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = InsightConfig::new().sample_rate(0.5); // 50% sampling
    /// ```
    pub fn sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Set the maximum body size to capture.
    ///
    /// Bodies larger than this will be truncated.
    pub fn max_body_size(mut self, size: usize) -> Self {
        self.max_body_size = size;
        self
    }

    /// Add a path to skip from insight collection.
    ///
    /// Exact match against request path.
    pub fn skip_path(mut self, path: impl Into<String>) -> Self {
        self.skip_paths.insert(path.into());
        self
    }

    /// Add multiple paths to skip.
    pub fn skip_paths(mut self, paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for path in paths {
            self.skip_paths.insert(path.into());
        }
        self
    }

    /// Add a path prefix to skip.
    ///
    /// Any request path starting with this prefix will be skipped.
    pub fn skip_path_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.skip_path_prefixes.insert(prefix.into());
        self
    }

    /// Set the request header whitelist.
    ///
    /// Only headers in this list will be captured. Use "*" to capture all.
    /// Header names are case-insensitive.
    pub fn header_whitelist(
        mut self,
        headers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.header_whitelist = headers
            .into_iter()
            .map(|h| h.into().to_lowercase())
            .collect();
        self
    }

    /// Set the response header whitelist.
    pub fn response_header_whitelist(
        mut self,
        headers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.response_header_whitelist = headers
            .into_iter()
            .map(|h| h.into().to_lowercase())
            .collect();
        self
    }

    /// Enable or disable request body capture.
    ///
    /// When enabled, request bodies (up to max_body_size) will be stored.
    pub fn capture_request_body(mut self, capture: bool) -> Self {
        self.capture_request_body = capture;
        self
    }

    /// Enable or disable response body capture.
    pub fn capture_response_body(mut self, capture: bool) -> Self {
        self.capture_response_body = capture;
        self
    }

    /// Set a callback to invoke for each collected insight.
    ///
    /// Useful for custom processing, external logging, or real-time alerts.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = InsightConfig::new()
    ///     .on_insight(|insight| {
    ///         if insight.duration_ms > 1000 {
    ///             tracing::warn!("Slow request: {} {}ms", insight.path, insight.duration_ms);
    ///         }
    ///     });
    /// ```
    pub fn on_insight<F>(mut self, callback: F) -> Self
    where
        F: Fn(&InsightData) + Send + Sync + 'static,
    {
        self.on_insight = Some(Arc::new(callback));
        self
    }

    /// Set the dashboard endpoint path.
    ///
    /// Set to None to disable the dashboard endpoint.
    pub fn dashboard_path(mut self, path: Option<impl Into<String>>) -> Self {
        self.dashboard_path = path.map(|p| p.into());
        self
    }

    /// Set the stats endpoint path.
    ///
    /// Set to None to disable the stats endpoint.
    pub fn stats_path(mut self, path: Option<impl Into<String>>) -> Self {
        self.stats_path = path.map(|p| p.into());
        self
    }

    /// Set the in-memory store capacity.
    ///
    /// Older entries are evicted when capacity is reached.
    pub fn store_capacity(mut self, capacity: usize) -> Self {
        self.store_capacity = capacity;
        self
    }

    /// Add a sensitive header name.
    ///
    /// Values for these headers will be replaced with `"[REDACTED]"`.
    pub fn sensitive_header(mut self, header: impl Into<String>) -> Self {
        self.sensitive_headers.insert(header.into().to_lowercase());
        self
    }

    /// Set capturable content types.
    ///
    /// Bodies are only captured for requests/responses with these content types.
    pub fn capturable_content_types(
        mut self,
        types: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.capturable_content_types =
            types.into_iter().map(|t| t.into().to_lowercase()).collect();
        self
    }

    /// Check if a path should be skipped.
    pub(crate) fn should_skip_path(&self, path: &str) -> bool {
        // Check exact matches
        if self.skip_paths.contains(path) {
            return true;
        }

        // Check prefixes
        for prefix in &self.skip_path_prefixes {
            if path.starts_with(prefix) {
                return true;
            }
        }

        // Check if this is a dashboard/stats path
        if let Some(ref dashboard) = self.dashboard_path {
            if path == dashboard {
                return true;
            }
        }
        if let Some(ref stats) = self.stats_path {
            if path == stats {
                return true;
            }
        }

        false
    }

    /// Check if the request should be sampled.
    pub(crate) fn should_sample(&self) -> bool {
        if self.sample_rate >= 1.0 {
            return true;
        }
        if self.sample_rate <= 0.0 {
            return false;
        }
        rand_sample(self.sample_rate)
    }

    /// Check if a header should be captured.
    pub(crate) fn should_capture_header(&self, name: &str) -> bool {
        if self.header_whitelist.is_empty() {
            return false;
        }
        if self.header_whitelist.contains("*") {
            return true;
        }
        self.header_whitelist.contains(&name.to_lowercase())
    }

    /// Check if a response header should be captured.
    pub(crate) fn should_capture_response_header(&self, name: &str) -> bool {
        if self.response_header_whitelist.is_empty() {
            return false;
        }
        if self.response_header_whitelist.contains("*") {
            return true;
        }
        self.response_header_whitelist
            .contains(&name.to_lowercase())
    }

    /// Check if a header is sensitive.
    pub(crate) fn is_sensitive_header(&self, name: &str) -> bool {
        self.sensitive_headers.contains(&name.to_lowercase())
    }

    /// Check if content type is capturable.
    pub(crate) fn is_capturable_content_type(&self, content_type: &str) -> bool {
        let ct_lower = content_type.to_lowercase();
        for allowed in &self.capturable_content_types {
            if ct_lower.starts_with(allowed)
                || (allowed.ends_with("/*") && ct_lower.starts_with(&allowed[..allowed.len() - 1]))
            {
                return true;
            }
        }
        // Also allow text/* generically
        ct_lower.starts_with("text/") || ct_lower.starts_with("application/json")
    }
}

/// Simple random sampling based on rate.
fn rand_sample(rate: f64) -> bool {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Use system time nanoseconds as a simple random source
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();

    let threshold = (rate * u32::MAX as f64) as u32;
    nanos < threshold
}

impl std::fmt::Debug for InsightConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InsightConfig")
            .field("sample_rate", &self.sample_rate)
            .field("max_body_size", &self.max_body_size)
            .field("skip_paths", &self.skip_paths)
            .field("skip_path_prefixes", &self.skip_path_prefixes)
            .field("header_whitelist", &self.header_whitelist)
            .field("capture_request_body", &self.capture_request_body)
            .field("capture_response_body", &self.capture_response_body)
            .field("dashboard_path", &self.dashboard_path)
            .field("stats_path", &self.stats_path)
            .field("store_capacity", &self.store_capacity)
            .field("on_insight", &self.on_insight.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = InsightConfig::new();
        assert_eq!(config.sample_rate, 1.0);
        assert_eq!(config.max_body_size, 4096);
        assert!(!config.capture_request_body);
        assert!(!config.capture_response_body);
        assert_eq!(config.dashboard_path, Some("/insights".to_string()));
        assert_eq!(config.stats_path, Some("/insights/stats".to_string()));
    }

    #[test]
    fn test_sample_rate_clamping() {
        let config = InsightConfig::new().sample_rate(1.5);
        assert_eq!(config.sample_rate, 1.0);

        let config = InsightConfig::new().sample_rate(-0.5);
        assert_eq!(config.sample_rate, 0.0);
    }

    #[test]
    fn test_skip_paths() {
        let config = InsightConfig::new()
            .skip_path("/health")
            .skip_path("/metrics")
            .skip_path_prefix("/internal/");

        assert!(config.should_skip_path("/health"));
        assert!(config.should_skip_path("/metrics"));
        assert!(config.should_skip_path("/internal/debug"));
        assert!(!config.should_skip_path("/users"));
    }

    #[test]
    fn test_header_whitelist() {
        let config = InsightConfig::new().header_whitelist(vec!["Content-Type", "User-Agent"]);

        assert!(config.should_capture_header("content-type"));
        assert!(config.should_capture_header("Content-Type"));
        assert!(config.should_capture_header("user-agent"));
        assert!(!config.should_capture_header("authorization"));
    }

    #[test]
    fn test_header_wildcard() {
        let config = InsightConfig::new().header_whitelist(vec!["*"]);

        assert!(config.should_capture_header("any-header"));
        assert!(config.should_capture_header("another-one"));
    }

    #[test]
    fn test_sensitive_headers() {
        let config = InsightConfig::new();

        assert!(config.is_sensitive_header("authorization"));
        assert!(config.is_sensitive_header("Authorization"));
        assert!(config.is_sensitive_header("cookie"));
        assert!(!config.is_sensitive_header("content-type"));
    }

    #[test]
    fn test_capturable_content_types() {
        let config = InsightConfig::new();

        assert!(config.is_capturable_content_type("application/json"));
        assert!(config.is_capturable_content_type("application/json; charset=utf-8"));
        assert!(config.is_capturable_content_type("text/plain"));
        assert!(config.is_capturable_content_type("text/html"));
    }

    #[test]
    fn test_dashboard_path_exclusion() {
        let config = InsightConfig::new()
            .dashboard_path(Some("/insights"))
            .stats_path(Some("/insights/stats"));

        assert!(config.should_skip_path("/insights"));
        assert!(config.should_skip_path("/insights/stats"));
        assert!(!config.should_skip_path("/users"));
    }
}
