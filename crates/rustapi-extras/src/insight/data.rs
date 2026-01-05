//! Data structures for traffic insight collection.
//!
//! This module defines the core data types used to capture and store
//! request/response information.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A single insight entry capturing request/response information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightData {
    /// Unique request identifier
    pub request_id: String,

    /// HTTP method (GET, POST, etc.)
    pub method: String,

    /// Request path (without query string)
    pub path: String,

    /// Query parameters as key-value pairs
    pub query_params: HashMap<String, String>,

    /// HTTP status code of the response
    pub status: u16,

    /// Request processing duration in milliseconds
    pub duration_ms: u64,

    /// Request body size in bytes
    pub request_size: usize,

    /// Response body size in bytes
    pub response_size: usize,

    /// Unix timestamp (seconds since epoch)
    pub timestamp: u64,

    /// Client IP address
    pub client_ip: String,

    /// Captured request headers (based on whitelist)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub request_headers: HashMap<String, String>,

    /// Captured response headers (based on whitelist)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub response_headers: HashMap<String, String>,

    /// Request body (if capture enabled and within size limit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,

    /// Response body (if capture enabled and within size limit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,

    /// Route pattern that matched (e.g., "/users/{id}")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_pattern: Option<String>,

    /// Custom tags/labels for categorization
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
}

impl InsightData {
    /// Create a new insight entry with required fields.
    pub fn new(
        request_id: impl Into<String>,
        method: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            method: method.into(),
            path: path.into(),
            query_params: HashMap::new(),
            status: 0,
            duration_ms: 0,
            request_size: 0,
            response_size: 0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            client_ip: String::new(),
            request_headers: HashMap::new(),
            response_headers: HashMap::new(),
            request_body: None,
            response_body: None,
            route_pattern: None,
            tags: HashMap::new(),
        }
    }

    /// Set the response status code.
    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Set the request duration.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = duration.as_millis() as u64;
        self
    }

    /// Set the client IP address.
    pub fn with_client_ip(mut self, ip: impl Into<String>) -> Self {
        self.client_ip = ip.into();
        self
    }

    /// Set request body size.
    pub fn with_request_size(mut self, size: usize) -> Self {
        self.request_size = size;
        self
    }

    /// Set response body size.
    pub fn with_response_size(mut self, size: usize) -> Self {
        self.response_size = size;
        self
    }

    /// Set route pattern.
    pub fn with_route_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.route_pattern = Some(pattern.into());
        self
    }

    /// Add a query parameter.
    pub fn add_query_param(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.query_params.insert(key.into(), value.into());
    }

    /// Add a request header.
    pub fn add_request_header(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.request_headers.insert(key.into(), value.into());
    }

    /// Add a response header.
    pub fn add_response_header(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.response_headers.insert(key.into(), value.into());
    }

    /// Set captured request body.
    pub fn set_request_body(&mut self, body: String) {
        self.request_body = Some(body);
    }

    /// Set captured response body.
    pub fn set_response_body(&mut self, body: String) {
        self.response_body = Some(body);
    }

    /// Add a custom tag.
    pub fn add_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }

    /// Check if this is a successful request (2xx status).
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Check if this is a client error (4xx status).
    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }

    /// Check if this is a server error (5xx status).
    pub fn is_server_error(&self) -> bool {
        self.status >= 500
    }
}

/// Aggregated statistics from collected insights.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InsightStats {
    /// Total number of requests
    pub total_requests: u64,

    /// Total number of successful requests (2xx)
    pub successful_requests: u64,

    /// Total number of client errors (4xx)
    pub client_errors: u64,

    /// Total number of server errors (5xx)
    pub server_errors: u64,

    /// Average response time in milliseconds
    pub avg_duration_ms: f64,

    /// Minimum response time in milliseconds
    pub min_duration_ms: u64,

    /// Maximum response time in milliseconds
    pub max_duration_ms: u64,

    /// 95th percentile response time in milliseconds
    pub p95_duration_ms: u64,

    /// 99th percentile response time in milliseconds
    pub p99_duration_ms: u64,

    /// Total bytes received (request bodies)
    pub total_request_bytes: u64,

    /// Total bytes sent (response bodies)
    pub total_response_bytes: u64,

    /// Requests per route pattern
    pub requests_by_route: HashMap<String, u64>,

    /// Requests per HTTP method
    pub requests_by_method: HashMap<String, u64>,

    /// Requests per status code
    pub requests_by_status: HashMap<u16, u64>,

    /// Average duration per route
    pub avg_duration_by_route: HashMap<String, f64>,

    /// Request rate (requests per second) over the measurement period
    pub requests_per_second: f64,

    /// Time period covered by these stats (in seconds)
    pub time_period_secs: u64,
}

impl InsightStats {
    /// Create new empty statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate statistics from a collection of insights.
    pub fn from_insights(insights: &[InsightData]) -> Self {
        if insights.is_empty() {
            return Self::default();
        }

        let mut stats = Self::new();
        stats.total_requests = insights.len() as u64;

        let mut durations: Vec<u64> = Vec::with_capacity(insights.len());
        let mut route_durations: HashMap<String, Vec<u64>> = HashMap::new();

        // Find time range
        let min_timestamp = insights.iter().map(|i| i.timestamp).min().unwrap_or(0);
        let max_timestamp = insights.iter().map(|i| i.timestamp).max().unwrap_or(0);
        stats.time_period_secs = max_timestamp.saturating_sub(min_timestamp).max(1);

        for insight in insights {
            // Count by status
            if insight.is_success() {
                stats.successful_requests += 1;
            } else if insight.is_client_error() {
                stats.client_errors += 1;
            } else if insight.is_server_error() {
                stats.server_errors += 1;
            }

            // Duration tracking
            durations.push(insight.duration_ms);

            // Bytes tracking
            stats.total_request_bytes += insight.request_size as u64;
            stats.total_response_bytes += insight.response_size as u64;

            // Route tracking
            let route = insight
                .route_pattern
                .clone()
                .unwrap_or_else(|| insight.path.clone());
            *stats.requests_by_route.entry(route.clone()).or_insert(0) += 1;
            route_durations
                .entry(route)
                .or_default()
                .push(insight.duration_ms);

            // Method tracking
            *stats
                .requests_by_method
                .entry(insight.method.clone())
                .or_insert(0) += 1;

            // Status tracking
            *stats.requests_by_status.entry(insight.status).or_insert(0) += 1;
        }

        // Calculate duration statistics
        if !durations.is_empty() {
            durations.sort_unstable();

            let sum: u64 = durations.iter().sum();
            stats.avg_duration_ms = sum as f64 / durations.len() as f64;
            stats.min_duration_ms = durations[0];
            stats.max_duration_ms = durations[durations.len() - 1];
            stats.p95_duration_ms = percentile(&durations, 95);
            stats.p99_duration_ms = percentile(&durations, 99);
        }

        // Calculate average duration per route
        for (route, route_durs) in route_durations {
            let sum: u64 = route_durs.iter().sum();
            let avg = sum as f64 / route_durs.len() as f64;
            stats.avg_duration_by_route.insert(route, avg);
        }

        // Calculate requests per second
        stats.requests_per_second = stats.total_requests as f64 / stats.time_period_secs as f64;

        stats
    }
}

/// Calculate the nth percentile of a sorted slice.
fn percentile(sorted: &[u64], n: u8) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (sorted.len() as f64 * (n as f64 / 100.0)).ceil() as usize;
    sorted[idx.saturating_sub(1).min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insight_data_creation() {
        let insight = InsightData::new("req-123", "GET", "/users")
            .with_status(200)
            .with_duration(Duration::from_millis(42))
            .with_client_ip("192.168.1.1");

        assert_eq!(insight.request_id, "req-123");
        assert_eq!(insight.method, "GET");
        assert_eq!(insight.path, "/users");
        assert_eq!(insight.status, 200);
        assert_eq!(insight.duration_ms, 42);
        assert_eq!(insight.client_ip, "192.168.1.1");
    }

    #[test]
    fn test_status_categorization() {
        assert!(InsightData::new("", "", "").with_status(200).is_success());
        assert!(InsightData::new("", "", "").with_status(201).is_success());
        assert!(InsightData::new("", "", "")
            .with_status(404)
            .is_client_error());
        assert!(InsightData::new("", "", "")
            .with_status(500)
            .is_server_error());
    }

    #[test]
    fn test_stats_calculation() {
        let insights = vec![
            InsightData::new("1", "GET", "/users")
                .with_status(200)
                .with_duration(Duration::from_millis(10)),
            InsightData::new("2", "POST", "/users")
                .with_status(201)
                .with_duration(Duration::from_millis(20)),
            InsightData::new("3", "GET", "/users")
                .with_status(404)
                .with_duration(Duration::from_millis(5)),
            InsightData::new("4", "GET", "/items")
                .with_status(500)
                .with_duration(Duration::from_millis(100)),
        ];

        let stats = InsightStats::from_insights(&insights);

        assert_eq!(stats.total_requests, 4);
        assert_eq!(stats.successful_requests, 2);
        assert_eq!(stats.client_errors, 1);
        assert_eq!(stats.server_errors, 1);
        assert_eq!(stats.requests_by_method.get("GET"), Some(&3));
        assert_eq!(stats.requests_by_method.get("POST"), Some(&1));
    }

    #[test]
    fn test_percentile_calculation() {
        let sorted = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile(&sorted, 50), 5);
        assert_eq!(percentile(&sorted, 95), 10);
        assert_eq!(percentile(&sorted, 99), 10);
    }
}
