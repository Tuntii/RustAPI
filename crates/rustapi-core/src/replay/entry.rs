//! Core data structures for replay entries.
//!
//! A [`ReplayEntry`] captures a complete HTTP request/response pair
//! for later replay and diff analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::meta::ReplayMeta;

/// Unique identifier for a replay entry.
pub type ReplayId = String;

/// A recorded HTTP request/response pair for replay debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEntry {
    /// Unique replay entry identifier (UUID v4).
    pub id: ReplayId,

    /// When this entry was recorded (Unix timestamp in milliseconds).
    pub recorded_at: u64,

    /// The recorded HTTP request.
    pub request: RecordedRequest,

    /// The recorded HTTP response.
    pub response: RecordedResponse,

    /// Additional metadata (route pattern, duration, tags).
    pub meta: ReplayMeta,
}

/// A recorded HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedRequest {
    /// HTTP method (GET, POST, etc.).
    pub method: String,

    /// Full URI including query string.
    pub uri: String,

    /// Request path (without query string).
    pub path: String,

    /// Query string (without leading `?`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Request headers (after redaction).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,

    /// Request body (after redaction and truncation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,

    /// Original request body size in bytes.
    pub body_size: usize,

    /// Whether the body was truncated.
    #[serde(default)]
    pub body_truncated: bool,
}

/// A recorded HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedResponse {
    /// HTTP status code.
    pub status: u16,

    /// Response headers (after redaction).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,

    /// Response body (after redaction and truncation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,

    /// Original response body size in bytes.
    pub body_size: usize,

    /// Whether the body was truncated.
    #[serde(default)]
    pub body_truncated: bool,
}

impl ReplayEntry {
    /// Create a new replay entry with a generated UUID and current timestamp.
    pub fn new(request: RecordedRequest, response: RecordedResponse, meta: ReplayMeta) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            recorded_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            request,
            response,
            meta,
        }
    }
}

impl RecordedRequest {
    /// Create a new recorded request.
    pub fn new(method: impl Into<String>, uri: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            uri: uri.into(),
            path: path.into(),
            query: None,
            headers: HashMap::new(),
            body: None,
            body_size: 0,
            body_truncated: false,
        }
    }
}

impl RecordedResponse {
    /// Create a new recorded response.
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: None,
            body_size: 0,
            body_truncated: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_entry_creation() {
        let req = RecordedRequest::new("GET", "/users?page=1", "/users");
        let resp = RecordedResponse::new(200);
        let meta = ReplayMeta::new().with_duration_ms(42);

        let entry = ReplayEntry::new(req, resp, meta);

        assert!(!entry.id.is_empty());
        assert!(entry.recorded_at > 0);
        assert_eq!(entry.request.method, "GET");
        assert_eq!(entry.request.uri, "/users?page=1");
        assert_eq!(entry.request.path, "/users");
        assert_eq!(entry.response.status, 200);
        assert_eq!(entry.meta.duration_ms, 42);
    }

    #[test]
    fn test_recorded_request_defaults() {
        let req = RecordedRequest::new("POST", "/items", "/items");
        assert_eq!(req.method, "POST");
        assert!(req.query.is_none());
        assert!(req.headers.is_empty());
        assert!(req.body.is_none());
        assert_eq!(req.body_size, 0);
        assert!(!req.body_truncated);
    }

    #[test]
    fn test_recorded_response_defaults() {
        let resp = RecordedResponse::new(404);
        assert_eq!(resp.status, 404);
        assert!(resp.headers.is_empty());
        assert!(resp.body.is_none());
        assert_eq!(resp.body_size, 0);
        assert!(!resp.body_truncated);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let req = RecordedRequest {
            method: "POST".to_string(),
            uri: "/api/users".to_string(),
            path: "/api/users".to_string(),
            query: None,
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h
            },
            body: Some(r#"{"name":"test"}"#.to_string()),
            body_size: 15,
            body_truncated: false,
        };
        let resp = RecordedResponse {
            status: 201,
            headers: HashMap::new(),
            body: Some(r#"{"id":1}"#.to_string()),
            body_size: 8,
            body_truncated: false,
        };
        let entry = ReplayEntry::new(req, resp, ReplayMeta::default());

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ReplayEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, entry.id);
        assert_eq!(deserialized.request.method, "POST");
        assert_eq!(deserialized.response.status, 201);
        assert_eq!(
            deserialized.request.body.as_deref(),
            Some(r#"{"name":"test"}"#)
        );
    }
}
