//! Metadata associated with a replay entry.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata associated with a replay entry.
///
/// Contains contextual information about the recorded request such as
/// route pattern, processing duration, client IP, and custom tags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplayMeta {
    /// Route pattern that matched (e.g., `"/users/{id}"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_pattern: Option<String>,

    /// Request processing duration in milliseconds.
    pub duration_ms: u64,

    /// Client IP address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_ip: Option<String>,

    /// Request ID for correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Custom tags for categorization and filtering.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,

    /// Time-to-live in seconds (for retention).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u64>,
}

impl ReplayMeta {
    /// Create a new empty metadata instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the route pattern.
    pub fn with_route_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.route_pattern = Some(pattern.into());
        self
    }

    /// Set the processing duration in milliseconds.
    pub fn with_duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Set the client IP address.
    pub fn with_client_ip(mut self, ip: impl Into<String>) -> Self {
        self.client_ip = Some(ip.into());
        self
    }

    /// Set the request ID.
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Add a custom tag.
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Set the TTL in seconds.
    pub fn with_ttl_secs(mut self, secs: u64) -> Self {
        self.ttl_secs = Some(secs);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let meta = ReplayMeta::new();
        assert!(meta.route_pattern.is_none());
        assert_eq!(meta.duration_ms, 0);
        assert!(meta.client_ip.is_none());
        assert!(meta.request_id.is_none());
        assert!(meta.tags.is_empty());
        assert!(meta.ttl_secs.is_none());
    }

    #[test]
    fn test_builder() {
        let meta = ReplayMeta::new()
            .with_route_pattern("/users/{id}")
            .with_duration_ms(42)
            .with_client_ip("192.168.1.1")
            .with_request_id("req-123")
            .with_tag("env", "staging")
            .with_ttl_secs(3600);

        assert_eq!(meta.route_pattern.as_deref(), Some("/users/{id}"));
        assert_eq!(meta.duration_ms, 42);
        assert_eq!(meta.client_ip.as_deref(), Some("192.168.1.1"));
        assert_eq!(meta.request_id.as_deref(), Some("req-123"));
        assert_eq!(meta.tags.get("env").map(|s| s.as_str()), Some("staging"));
        assert_eq!(meta.ttl_secs, Some(3600));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let meta = ReplayMeta::new()
            .with_route_pattern("/test")
            .with_duration_ms(100);

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: ReplayMeta = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.route_pattern, meta.route_pattern);
        assert_eq!(deserialized.duration_ms, meta.duration_ms);
    }
}
