//! Storage backends for traffic insight data.
//!
//! This module provides the `InsightStore` trait and default implementations
//! for storing and retrieving insight data.

use super::data::{InsightData, InsightStats};
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for storing and retrieving insight data.
///
/// Implement this trait to create custom storage backends (e.g., database, Redis).
#[async_trait]
pub trait InsightStore: Send + Sync + 'static {
    /// Store a new insight entry.
    async fn store(&self, insight: InsightData);

    /// Get recent insights (up to `limit` entries).
    async fn get_recent(&self, limit: usize) -> Vec<InsightData>;

    /// Get all stored insights.
    async fn get_all(&self) -> Vec<InsightData>;

    /// Get insights filtered by path pattern.
    async fn get_by_path(&self, path_pattern: &str) -> Vec<InsightData>;

    /// Get insights filtered by status code range.
    async fn get_by_status(&self, min_status: u16, max_status: u16) -> Vec<InsightData>;

    /// Get aggregated statistics.
    async fn get_stats(&self) -> InsightStats;

    /// Clear all stored insights.
    async fn clear(&self);

    /// Get the current count of stored insights.
    async fn count(&self) -> usize;

    /// Clone this store into a boxed trait object.
    fn clone_store(&self) -> Box<dyn InsightStore>;
}

/// In-memory insight store using a ring buffer.
///
/// This store keeps the most recent N insights in memory with thread-safe access.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::insight::InMemoryInsightStore;
///
/// // Store up to 1000 insights
/// let store = InMemoryInsightStore::new(1000);
/// ```
#[derive(Clone)]
pub struct InMemoryInsightStore {
    /// Ring buffer holding insights (in order)
    buffer: Arc<RwLock<VecDeque<Arc<InsightData>>>>,
    /// Maximum capacity of the buffer
    capacity: usize,
    /// Index for quick lookup by request_id
    index: Arc<DashMap<String, Arc<InsightData>>>,
}

impl InMemoryInsightStore {
    /// Create a new in-memory store with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of insights to store (default: 1000)
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            capacity,
            index: Arc::new(DashMap::new()),
        }
    }

    /// Create a new in-memory store with default capacity (1000 entries).
    pub fn default_capacity() -> Self {
        Self::new(1000)
    }

    /// Get the maximum capacity of this store.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get an insight by request ID.
    pub fn get_by_request_id(&self, request_id: &str) -> Option<InsightData> {
        // Look up directly in index (O(1))
        self.index.get(request_id).map(|r| r.as_ref().clone())
    }
}

impl Default for InMemoryInsightStore {
    fn default() -> Self {
        Self::default_capacity()
    }
}

#[async_trait]
impl InsightStore for InMemoryInsightStore {
    async fn store(&self, insight: InsightData) {
        let insight_arc = Arc::new(insight);
        let request_id = insight_arc.request_id.clone();

        let mut buffer = self.buffer.write().await;

        // If at capacity, remove oldest entry
        if buffer.len() >= self.capacity {
            if let Some(old) = buffer.pop_front() {
                self.index.remove(&old.request_id);
            }
        }

        // Add new insight
        buffer.push_back(insight_arc.clone());
        self.index.insert(request_id, insight_arc);
    }

    async fn get_recent(&self, limit: usize) -> Vec<InsightData> {
        let buffer = self.buffer.read().await;
        buffer.iter().rev().take(limit).map(|i| i.as_ref().clone()).collect()
    }

    async fn get_all(&self) -> Vec<InsightData> {
        let buffer = self.buffer.read().await;
        buffer.iter().map(|i| i.as_ref().clone()).collect()
    }

    async fn get_by_path(&self, path_pattern: &str) -> Vec<InsightData> {
        let buffer = self.buffer.read().await;
        buffer
            .iter()
            .filter(|i| i.path.contains(path_pattern))
            .map(|i| i.as_ref().clone())
            .collect()
    }

    async fn get_by_status(&self, min_status: u16, max_status: u16) -> Vec<InsightData> {
        let buffer = self.buffer.read().await;
        buffer
            .iter()
            .filter(|i| i.status >= min_status && i.status <= max_status)
            .map(|i| i.as_ref().clone())
            .collect()
    }

    async fn get_stats(&self) -> InsightStats {
        let all = self.get_all().await;
        InsightStats::from_insights(&all)
    }

    async fn clear(&self) {
        let mut buffer = self.buffer.write().await;
        buffer.clear();
        self.index.clear();
    }

    async fn count(&self) -> usize {
        self.buffer.read().await.len()
    }

    fn clone_store(&self) -> Box<dyn InsightStore> {
        Box::new(self.clone())
    }
}

/// A no-op store that discards all insights.
///
/// Useful for testing or when you only want callback-based processing.
#[derive(Clone, Copy, Default)]
pub struct NullInsightStore;

#[async_trait]
impl InsightStore for NullInsightStore {
    async fn store(&self, _insight: InsightData) {
        // Discard
    }

    async fn get_recent(&self, _limit: usize) -> Vec<InsightData> {
        Vec::new()
    }

    async fn get_all(&self) -> Vec<InsightData> {
        Vec::new()
    }

    async fn get_by_path(&self, _path_pattern: &str) -> Vec<InsightData> {
        Vec::new()
    }

    async fn get_by_status(&self, _min_status: u16, _max_status: u16) -> Vec<InsightData> {
        Vec::new()
    }

    async fn get_stats(&self) -> InsightStats {
        InsightStats::default()
    }

    async fn clear(&self) {}

    async fn count(&self) -> usize {
        0
    }

    fn clone_store(&self) -> Box<dyn InsightStore> {
        Box::new(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_insight(id: &str, path: &str, status: u16) -> InsightData {
        InsightData::new(id, "GET", path)
            .with_status(status)
            .with_duration(Duration::from_millis(10))
    }

    #[tokio::test]
    async fn test_in_memory_store_basic() {
        let store = InMemoryInsightStore::new(10);

        store.store(create_test_insight("1", "/users", 200)).await;
        store.store(create_test_insight("2", "/items", 201)).await;

        assert_eq!(store.count().await, 2);

        let recent = store.get_recent(10).await;
        assert_eq!(recent.len(), 2);
        // Most recent first
        assert_eq!(recent[0].request_id, "2");
        assert_eq!(recent[1].request_id, "1");
    }

    #[tokio::test]
    async fn test_ring_buffer_eviction() {
        let store = InMemoryInsightStore::new(3);

        store.store(create_test_insight("1", "/a", 200)).await;
        store.store(create_test_insight("2", "/b", 200)).await;
        store.store(create_test_insight("3", "/c", 200)).await;
        store.store(create_test_insight("4", "/d", 200)).await; // Should evict "1"

        assert_eq!(store.count().await, 3);

        let all = store.get_all().await;
        let ids: Vec<_> = all.iter().map(|i| i.request_id.as_str()).collect();
        assert!(!ids.contains(&"1"));
        assert!(ids.contains(&"2"));
        assert!(ids.contains(&"3"));
        assert!(ids.contains(&"4"));
    }

    #[tokio::test]
    async fn test_filter_by_path() {
        let store = InMemoryInsightStore::new(10);

        store.store(create_test_insight("1", "/users/123", 200)).await;
        store.store(create_test_insight("2", "/items/456", 200)).await;
        store.store(create_test_insight("3", "/users/789", 200)).await;

        let user_insights = store.get_by_path("/users").await;
        assert_eq!(user_insights.len(), 2);
    }

    #[tokio::test]
    async fn test_filter_by_status() {
        let store = InMemoryInsightStore::new(10);

        store.store(create_test_insight("1", "/a", 200)).await;
        store.store(create_test_insight("2", "/b", 404)).await;
        store.store(create_test_insight("3", "/c", 500)).await;
        store.store(create_test_insight("4", "/d", 201)).await;

        let errors = store.get_by_status(400, 599).await;
        assert_eq!(errors.len(), 2);

        let success = store.get_by_status(200, 299).await;
        assert_eq!(success.len(), 2);
    }

    #[tokio::test]
    async fn test_clear() {
        let store = InMemoryInsightStore::new(10);

        store.store(create_test_insight("1", "/a", 200)).await;
        store.store(create_test_insight("2", "/b", 200)).await;

        assert_eq!(store.count().await, 2);

        store.clear().await;

        assert_eq!(store.count().await, 0);
        assert!(store.get_all().await.is_empty());
    }

    #[tokio::test]
    async fn test_stats() {
        let store = InMemoryInsightStore::new(10);

        store.store(create_test_insight("1", "/users", 200)).await;
        store.store(create_test_insight("2", "/users", 201)).await;
        store.store(create_test_insight("3", "/items", 404)).await;

        let stats = store.get_stats().await;

        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.successful_requests, 2);
        assert_eq!(stats.client_errors, 1);
    }

    #[tokio::test]
    async fn test_null_store() {
        let store = NullInsightStore;

        store.store(create_test_insight("1", "/a", 200)).await;

        assert_eq!(store.count().await, 0);
        assert!(store.get_all().await.is_empty());
    }
}
