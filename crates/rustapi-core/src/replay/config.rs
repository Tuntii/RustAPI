//! Configuration for the replay recording system.
//!
//! Provides [`ReplayConfig`] with a builder pattern for customizing
//! replay behavior. Secure defaults: disabled, admin token required,
//! sensitive headers redacted, TTL enforced.

use std::collections::HashSet;

/// Configuration for the replay recording middleware.
///
/// Uses builder pattern with secure defaults:
/// - Recording disabled (`enabled: false`)
/// - Admin token required for replay endpoints
/// - Sensitive headers redacted by default
/// - TTL enforced (1 hour default)
///
/// # Example
///
/// ```ignore
/// use rustapi_core::replay::ReplayConfig;
///
/// let config = ReplayConfig::new()
///     .enabled(true)
///     .admin_token("my-secret-token")
///     .ttl_secs(3600)
///     .redact_header("x-custom-secret");
/// ```
#[derive(Clone)]
pub struct ReplayConfig {
    /// Whether replay recording is enabled. Default: false (off in production).
    pub enabled: bool,

    /// Admin bearer token required for replay endpoints. Must be set.
    pub admin_token: Option<String>,

    /// Paths to record (empty = all paths). Mutually exclusive with skip_paths.
    pub record_paths: HashSet<String>,

    /// Paths to skip from recording.
    pub skip_paths: HashSet<String>,

    /// Path prefix for admin routes. Default: `"/__rustapi/replays"`.
    pub admin_route_prefix: String,

    /// Maximum request body size to capture (bytes). Default: 64KB.
    pub max_request_body: usize,

    /// Maximum response body size to capture (bytes). Default: 256KB.
    pub max_response_body: usize,

    /// Store capacity (in-memory ring buffer size). Default: 500.
    pub store_capacity: usize,

    /// Time-to-live for entries in seconds. Default: 3600 (1 hour).
    pub ttl_secs: u64,

    /// Sampling rate (0.0-1.0). Default: 1.0 (all requests).
    pub sample_rate: f64,

    /// Headers to redact (values replaced with `[REDACTED]`).
    pub redact_headers: HashSet<String>,

    /// JSON body field paths to redact.
    pub redact_body_fields: HashSet<String>,

    /// Content types eligible for body capture.
    pub capturable_content_types: HashSet<String>,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayConfig {
    /// Create a new configuration with secure defaults.
    ///
    /// Defaults:
    /// - Replay disabled
    /// - No admin token (must be set before use)
    /// - Max request body: 64KB
    /// - Max response body: 256KB
    /// - Store capacity: 500 entries
    /// - TTL: 3600 seconds (1 hour)
    /// - Sample rate: 1.0 (all requests)
    /// - Redacted headers: authorization, cookie, x-api-key, x-auth-token
    pub fn new() -> Self {
        let mut redact_headers = HashSet::new();
        redact_headers.insert("authorization".to_string());
        redact_headers.insert("cookie".to_string());
        redact_headers.insert("x-api-key".to_string());
        redact_headers.insert("x-auth-token".to_string());

        let mut capturable = HashSet::new();
        capturable.insert("application/json".to_string());
        capturable.insert("text/plain".to_string());
        capturable.insert("text/html".to_string());
        capturable.insert("application/xml".to_string());
        capturable.insert("text/xml".to_string());

        Self {
            enabled: false,
            admin_token: None,
            record_paths: HashSet::new(),
            skip_paths: HashSet::new(),
            admin_route_prefix: "/__rustapi/replays".to_string(),
            max_request_body: 65_536,  // 64KB
            max_response_body: 262_144, // 256KB
            store_capacity: 500,
            ttl_secs: 3600,
            sample_rate: 1.0,
            redact_headers,
            redact_body_fields: HashSet::new(),
            capturable_content_types: capturable,
        }
    }

    /// Enable or disable replay recording.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the admin bearer token for replay endpoints.
    ///
    /// All `/__rustapi/replays` endpoints require this token
    /// in the `Authorization: Bearer <token>` header.
    pub fn admin_token(mut self, token: impl Into<String>) -> Self {
        self.admin_token = Some(token.into());
        self
    }

    /// Add a path to record. If any record paths are set,
    /// only those paths will be recorded.
    pub fn record_path(mut self, path: impl Into<String>) -> Self {
        self.record_paths.insert(path.into());
        self
    }

    /// Add a path to skip from recording.
    pub fn skip_path(mut self, path: impl Into<String>) -> Self {
        self.skip_paths.insert(path.into());
        self
    }

    /// Set the admin route prefix. Default: `"/__rustapi/replays"`.
    pub fn admin_route_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.admin_route_prefix = prefix.into();
        self
    }

    /// Set the maximum request body size to capture (bytes).
    pub fn max_request_body(mut self, size: usize) -> Self {
        self.max_request_body = size;
        self
    }

    /// Set the maximum response body size to capture (bytes).
    pub fn max_response_body(mut self, size: usize) -> Self {
        self.max_response_body = size;
        self
    }

    /// Set the store capacity (max number of entries).
    pub fn store_capacity(mut self, capacity: usize) -> Self {
        self.store_capacity = capacity;
        self
    }

    /// Set the TTL for replay entries (seconds).
    pub fn ttl_secs(mut self, secs: u64) -> Self {
        self.ttl_secs = secs;
        self
    }

    /// Set the sampling rate (0.0 to 1.0).
    pub fn sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Add a header name to redact (case-insensitive).
    pub fn redact_header(mut self, header: impl Into<String>) -> Self {
        self.redact_headers.insert(header.into().to_lowercase());
        self
    }

    /// Add a JSON body field path to redact (e.g., `"password"`, `"ssn"`).
    pub fn redact_body_field(mut self, field: impl Into<String>) -> Self {
        self.redact_body_fields.insert(field.into());
        self
    }

    /// Add a capturable content type.
    pub fn capturable_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.capturable_content_types
            .insert(content_type.into().to_lowercase());
        self
    }

    /// Check if a path should be recorded.
    pub fn should_record_path(&self, path: &str) -> bool {
        // Skip admin routes
        if path.starts_with(&self.admin_route_prefix) {
            return false;
        }

        // Skip explicitly skipped paths
        if self.skip_paths.contains(path) {
            return false;
        }

        // If record_paths is set, only record those
        if !self.record_paths.is_empty() {
            return self.record_paths.contains(path);
        }

        true
    }

    /// Check if this request should be sampled.
    pub fn should_sample(&self) -> bool {
        if self.sample_rate >= 1.0 {
            return true;
        }
        if self.sample_rate <= 0.0 {
            return false;
        }
        rand_sample(self.sample_rate)
    }

    /// Check if a content type is capturable.
    pub fn is_capturable_content_type(&self, content_type: &str) -> bool {
        let ct_lower = content_type.to_lowercase();
        for allowed in &self.capturable_content_types {
            if ct_lower.starts_with(allowed) {
                return true;
            }
        }
        ct_lower.starts_with("text/") || ct_lower.starts_with("application/json")
    }
}

/// Simple random sampling based on rate.
fn rand_sample(rate: f64) -> bool {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();

    let threshold = (rate * u32::MAX as f64) as u32;
    nanos < threshold
}

impl std::fmt::Debug for ReplayConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplayConfig")
            .field("enabled", &self.enabled)
            .field("admin_token", &self.admin_token.as_ref().map(|_| "[SET]"))
            .field("record_paths", &self.record_paths)
            .field("skip_paths", &self.skip_paths)
            .field("admin_route_prefix", &self.admin_route_prefix)
            .field("max_request_body", &self.max_request_body)
            .field("max_response_body", &self.max_response_body)
            .field("store_capacity", &self.store_capacity)
            .field("ttl_secs", &self.ttl_secs)
            .field("sample_rate", &self.sample_rate)
            .field("redact_headers", &self.redact_headers)
            .field("redact_body_fields", &self.redact_body_fields)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ReplayConfig::new();
        assert!(!config.enabled);
        assert!(config.admin_token.is_none());
        assert_eq!(config.max_request_body, 65_536);
        assert_eq!(config.max_response_body, 262_144);
        assert_eq!(config.store_capacity, 500);
        assert_eq!(config.ttl_secs, 3600);
        assert_eq!(config.sample_rate, 1.0);
        assert_eq!(config.admin_route_prefix, "/__rustapi/replays");
    }

    #[test]
    fn test_default_redacted_headers() {
        let config = ReplayConfig::new();
        assert!(config.redact_headers.contains("authorization"));
        assert!(config.redact_headers.contains("cookie"));
        assert!(config.redact_headers.contains("x-api-key"));
        assert!(config.redact_headers.contains("x-auth-token"));
    }

    #[test]
    fn test_builder_methods() {
        let config = ReplayConfig::new()
            .enabled(true)
            .admin_token("test-token")
            .max_request_body(1024)
            .max_response_body(2048)
            .store_capacity(100)
            .ttl_secs(7200)
            .sample_rate(0.5)
            .redact_header("x-custom")
            .redact_body_field("password")
            .record_path("/api/users")
            .skip_path("/health");

        assert!(config.enabled);
        assert_eq!(config.admin_token.as_deref(), Some("test-token"));
        assert_eq!(config.max_request_body, 1024);
        assert_eq!(config.max_response_body, 2048);
        assert_eq!(config.store_capacity, 100);
        assert_eq!(config.ttl_secs, 7200);
        assert_eq!(config.sample_rate, 0.5);
        assert!(config.redact_headers.contains("x-custom"));
        assert!(config.redact_body_fields.contains("password"));
        assert!(config.record_paths.contains("/api/users"));
        assert!(config.skip_paths.contains("/health"));
    }

    #[test]
    fn test_sample_rate_clamping() {
        let config = ReplayConfig::new().sample_rate(1.5);
        assert_eq!(config.sample_rate, 1.0);

        let config = ReplayConfig::new().sample_rate(-0.5);
        assert_eq!(config.sample_rate, 0.0);
    }

    #[test]
    fn test_should_record_path() {
        let config = ReplayConfig::new()
            .skip_path("/health")
            .record_path("/api/users");

        assert!(!config.should_record_path("/__rustapi/replays"));
        assert!(!config.should_record_path("/health"));
        assert!(config.should_record_path("/api/users"));
        assert!(!config.should_record_path("/api/items"));
    }

    #[test]
    fn test_should_record_path_no_record_filter() {
        let config = ReplayConfig::new().skip_path("/health");

        assert!(config.should_record_path("/api/users"));
        assert!(config.should_record_path("/api/items"));
        assert!(!config.should_record_path("/health"));
    }

    #[test]
    fn test_capturable_content_type() {
        let config = ReplayConfig::new();

        assert!(config.is_capturable_content_type("application/json"));
        assert!(config.is_capturable_content_type("application/json; charset=utf-8"));
        assert!(config.is_capturable_content_type("text/plain"));
        assert!(config.is_capturable_content_type("text/html"));
    }
}
