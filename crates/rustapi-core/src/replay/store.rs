//! Storage trait and query types for replay entries.
//!
//! Defines the [`ReplayStore`] trait for pluggable storage backends.

use async_trait::async_trait;

use super::entry::ReplayEntry;

/// Errors from replay store operations.
#[derive(Debug, thiserror::Error)]
pub enum ReplayStoreError {
    /// IO error (file, network, etc.).
    #[error("IO error: {0}")]
    Io(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Entry not found.
    #[error("Entry not found: {0}")]
    NotFound(String),

    /// Store is full.
    #[error("Store full")]
    StoreFull,

    /// Other error.
    #[error("Store error: {0}")]
    Other(String),
}

/// Convenience result type for replay store operations.
pub type ReplayStoreResult<T> = Result<T, ReplayStoreError>;

/// Query parameters for filtering replay entries.
#[derive(Debug, Clone, Default)]
pub struct ReplayQuery {
    /// Filter by HTTP method.
    pub method: Option<String>,

    /// Filter by path substring.
    pub path_contains: Option<String>,

    /// Filter by minimum status code.
    pub status_min: Option<u16>,

    /// Filter by maximum status code.
    pub status_max: Option<u16>,

    /// Filter entries recorded after this timestamp (Unix ms).
    pub from_timestamp: Option<u64>,

    /// Filter entries recorded before this timestamp (Unix ms).
    pub to_timestamp: Option<u64>,

    /// Filter by tag key-value pair.
    pub tag: Option<(String, String)>,

    /// Maximum number of entries to return.
    pub limit: Option<usize>,

    /// Number of entries to skip.
    pub offset: Option<usize>,

    /// Return newest entries first. Default: true.
    pub newest_first: bool,
}

impl ReplayQuery {
    /// Create a new empty query (matches all entries).
    pub fn new() -> Self {
        Self {
            newest_first: true,
            ..Default::default()
        }
    }

    /// Filter by HTTP method.
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Filter by path substring.
    pub fn path_contains(mut self, path: impl Into<String>) -> Self {
        self.path_contains = Some(path.into());
        self
    }

    /// Filter by minimum status code.
    pub fn status_min(mut self, min: u16) -> Self {
        self.status_min = Some(min);
        self
    }

    /// Filter by maximum status code.
    pub fn status_max(mut self, max: u16) -> Self {
        self.status_max = Some(max);
        self
    }

    /// Set the maximum number of entries to return.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Check if an entry matches this query.
    pub fn matches(&self, entry: &ReplayEntry) -> bool {
        if let Some(ref method) = self.method {
            if entry.request.method != *method {
                return false;
            }
        }
        if let Some(ref path) = self.path_contains {
            if !entry.request.path.contains(path.as_str()) {
                return false;
            }
        }
        if let Some(min) = self.status_min {
            if entry.response.status < min {
                return false;
            }
        }
        if let Some(max) = self.status_max {
            if entry.response.status > max {
                return false;
            }
        }
        if let Some(from) = self.from_timestamp {
            if entry.recorded_at < from {
                return false;
            }
        }
        if let Some(to) = self.to_timestamp {
            if entry.recorded_at > to {
                return false;
            }
        }
        if let Some((ref key, ref value)) = self.tag {
            match entry.meta.tags.get(key) {
                Some(v) if v == value => {}
                _ => return false,
            }
        }
        true
    }
}

/// Trait for storing and retrieving replay entries.
///
/// Implement this trait to create custom storage backends
/// (e.g., database, Redis, cloud storage).
#[async_trait]
pub trait ReplayStore: Send + Sync + 'static {
    /// Store a new replay entry.
    async fn store(&self, entry: ReplayEntry) -> ReplayStoreResult<()>;

    /// Get a single replay entry by ID.
    async fn get(&self, id: &str) -> ReplayStoreResult<Option<ReplayEntry>>;

    /// List replay entries matching the given query.
    async fn list(&self, query: &ReplayQuery) -> ReplayStoreResult<Vec<ReplayEntry>>;

    /// Delete a replay entry by ID. Returns true if deleted.
    async fn delete(&self, id: &str) -> ReplayStoreResult<bool>;

    /// Get the total count of stored entries.
    async fn count(&self) -> ReplayStoreResult<usize>;

    /// Clear all stored entries.
    async fn clear(&self) -> ReplayStoreResult<()>;

    /// Delete entries recorded before the given timestamp (Unix ms).
    /// Returns the number of deleted entries.
    async fn delete_before(&self, timestamp_ms: u64) -> ReplayStoreResult<usize>;

    /// Clone this store into a boxed trait object.
    fn clone_store(&self) -> Box<dyn ReplayStore>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::entry::{RecordedRequest, RecordedResponse};
    use crate::replay::meta::ReplayMeta;

    fn make_entry(method: &str, path: &str, status: u16) -> ReplayEntry {
        ReplayEntry::new(
            RecordedRequest::new(method, path, path),
            RecordedResponse::new(status),
            ReplayMeta::new(),
        )
    }

    #[test]
    fn test_query_matches_all() {
        let query = ReplayQuery::new();
        let entry = make_entry("GET", "/users", 200);
        assert!(query.matches(&entry));
    }

    #[test]
    fn test_query_method_filter() {
        let query = ReplayQuery::new().method("POST");
        assert!(!query.matches(&make_entry("GET", "/users", 200)));
        assert!(query.matches(&make_entry("POST", "/users", 201)));
    }

    #[test]
    fn test_query_path_filter() {
        let query = ReplayQuery::new().path_contains("/users");
        assert!(query.matches(&make_entry("GET", "/users/123", 200)));
        assert!(!query.matches(&make_entry("GET", "/items", 200)));
    }

    #[test]
    fn test_query_status_filter() {
        let query = ReplayQuery::new().status_min(400).status_max(499);
        assert!(!query.matches(&make_entry("GET", "/a", 200)));
        assert!(query.matches(&make_entry("GET", "/a", 404)));
        assert!(!query.matches(&make_entry("GET", "/a", 500)));
    }

    #[test]
    fn test_query_limit() {
        let query = ReplayQuery::new().limit(10);
        assert_eq!(query.limit, Some(10));
    }
}
