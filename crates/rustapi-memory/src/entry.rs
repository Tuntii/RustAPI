use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single entry in the memory store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique key for this entry.
    pub key: String,
    /// The stored value.
    pub value: serde_json::Value,
    /// Optional vector embedding for semantic search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Namespace for logical grouping (e.g. session id, user id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// When this entry was created.
    pub created_at: DateTime<Utc>,
    /// When this entry was last updated.
    pub updated_at: DateTime<Utc>,
    /// Time-to-live. After this duration the entry should be considered expired.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u64>,
}

impl MemoryEntry {
    /// Create a new memory entry with the given key and value.
    pub fn new(key: impl Into<String>, value: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            key: key.into(),
            value,
            embedding: None,
            metadata: HashMap::new(),
            namespace: None,
            created_at: now,
            updated_at: now,
            ttl_secs: None,
        }
    }

    /// Builder: set namespace.
    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    /// Builder: set embedding vector.
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Builder: set TTL in seconds.
    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = Some(ttl_secs);
        self
    }

    /// Builder: add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Check whether this entry has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_secs {
            let elapsed = (Utc::now() - self.created_at).num_seconds();
            elapsed > ttl as i64
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryQuery — search / filter parameters
// ---------------------------------------------------------------------------

/// Parameters for querying the memory store.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Filter by namespace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Filter: key prefix match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_prefix: Option<String>,
    /// Filter: metadata key-value match.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata_filter: HashMap<String, serde_json::Value>,
    /// Maximum number of results.
    pub limit: usize,
    /// Offset for pagination.
    pub offset: usize,
    /// Whether to return newest first.
    pub newest_first: bool,
}

impl MemoryQuery {
    pub fn new() -> Self {
        Self {
            limit: 100,
            newest_first: true,
            ..Default::default()
        }
    }

    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(prefix.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Check whether a given entry matches this query's filters.
    pub fn matches(&self, entry: &MemoryEntry) -> bool {
        if let Some(ref ns) = self.namespace {
            if entry.namespace.as_deref() != Some(ns.as_str()) {
                return false;
            }
        }
        if let Some(ref prefix) = self.key_prefix {
            if !entry.key.starts_with(prefix.as_str()) {
                return false;
            }
        }
        for (k, v) in &self.metadata_filter {
            if entry.metadata.get(k) != Some(v) {
                return false;
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// SemanticQuery — vector similarity search
// ---------------------------------------------------------------------------

/// Parameters for semantic (vector similarity) search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticQuery {
    /// The query embedding vector.
    pub embedding: Vec<f32>,
    /// Maximum number of results.
    pub limit: usize,
    /// Minimum similarity threshold (0.0 – 1.0).
    pub min_similarity: f32,
    /// Optional namespace filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

impl SemanticQuery {
    pub fn new(embedding: Vec<f32>) -> Self {
        Self {
            embedding,
            limit: 10,
            min_similarity: 0.0,
            namespace: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_min_similarity(mut self, threshold: f32) -> Self {
        self.min_similarity = threshold;
        self
    }

    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }
}

/// A search result with similarity score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredEntry {
    /// The matching memory entry.
    pub entry: MemoryEntry,
    /// Cosine similarity score (0.0 – 1.0).
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_entry_basic() {
        let entry = MemoryEntry::new("key1", serde_json::json!({"data": "hello"}))
            .with_namespace("session-1")
            .with_ttl(3600);

        assert_eq!(entry.key, "key1");
        assert!(!entry.is_expired());
        assert_eq!(entry.namespace, Some("session-1".into()));
    }

    #[test]
    fn test_memory_query_matches() {
        let entry = MemoryEntry::new("user:42:prefs", serde_json::json!({}))
            .with_namespace("global")
            .with_metadata("type", serde_json::json!("preferences"));

        let query = MemoryQuery::new()
            .with_namespace("global")
            .with_key_prefix("user:42");
        assert!(query.matches(&entry));

        let wrong_ns = MemoryQuery::new().with_namespace("other");
        assert!(!wrong_ns.matches(&entry));
    }
}
