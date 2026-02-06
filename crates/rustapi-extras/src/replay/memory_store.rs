//! In-memory replay store using a ring buffer.
//!
//! Thread-safe, bounded storage with FIFO eviction.

use async_trait::async_trait;
use dashmap::DashMap;
use rustapi_core::replay::{
    ReplayEntry, ReplayQuery, ReplayStore, ReplayStoreResult,
};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory replay store with a bounded ring buffer.
///
/// When capacity is reached, the oldest entries are evicted.
/// Thread-safe via `RwLock` and `DashMap`.
///
/// # Example
///
/// ```ignore
/// use rustapi_extras::replay::InMemoryReplayStore;
///
/// let store = InMemoryReplayStore::new(500);
/// ```
#[derive(Clone)]
pub struct InMemoryReplayStore {
    buffer: Arc<RwLock<VecDeque<Arc<ReplayEntry>>>>,
    index: Arc<DashMap<String, Arc<ReplayEntry>>>,
    capacity: usize,
}

impl InMemoryReplayStore {
    /// Create a new in-memory store with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            index: Arc::new(DashMap::new()),
            capacity,
        }
    }
}

impl Default for InMemoryReplayStore {
    fn default() -> Self {
        Self::new(500)
    }
}

#[async_trait]
impl ReplayStore for InMemoryReplayStore {
    async fn store(&self, entry: ReplayEntry) -> ReplayStoreResult<()> {
        let entry_arc = Arc::new(entry);
        let id = entry_arc.id.clone();

        let mut buffer = self.buffer.write().await;

        // Evict oldest if at capacity
        if buffer.len() >= self.capacity {
            if let Some(old) = buffer.pop_front() {
                self.index.remove(&old.id);
            }
        }

        buffer.push_back(entry_arc.clone());
        self.index.insert(id, entry_arc);
        Ok(())
    }

    async fn get(&self, id: &str) -> ReplayStoreResult<Option<ReplayEntry>> {
        Ok(self.index.get(id).map(|r| r.as_ref().clone()))
    }

    async fn list(&self, query: &ReplayQuery) -> ReplayStoreResult<Vec<ReplayEntry>> {
        let buffer = self.buffer.read().await;

        let iter: Box<dyn Iterator<Item = &Arc<ReplayEntry>> + '_> = if query.newest_first {
            Box::new(buffer.iter().rev())
        } else {
            Box::new(buffer.iter())
        };

        let mut results: Vec<ReplayEntry> = iter
            .filter(|e| query.matches(e))
            .skip(query.offset.unwrap_or(0))
            .take(query.limit.unwrap_or(usize::MAX))
            .map(|e| e.as_ref().clone())
            .collect();

        // Ensure consistent ordering
        if !query.newest_first {
            results.reverse();
            results.reverse(); // already correct order
        }

        Ok(results)
    }

    async fn delete(&self, id: &str) -> ReplayStoreResult<bool> {
        let removed = self.index.remove(id).is_some();
        if removed {
            let mut buffer = self.buffer.write().await;
            buffer.retain(|e| e.id != id);
        }
        Ok(removed)
    }

    async fn count(&self) -> ReplayStoreResult<usize> {
        Ok(self.buffer.read().await.len())
    }

    async fn clear(&self) -> ReplayStoreResult<()> {
        let mut buffer = self.buffer.write().await;
        buffer.clear();
        self.index.clear();
        Ok(())
    }

    async fn delete_before(&self, timestamp_ms: u64) -> ReplayStoreResult<usize> {
        let mut buffer = self.buffer.write().await;
        let before_len = buffer.len();

        // Collect IDs to remove
        let to_remove: Vec<String> = buffer
            .iter()
            .filter(|e| e.recorded_at < timestamp_ms)
            .map(|e| e.id.clone())
            .collect();

        for id in &to_remove {
            self.index.remove(id);
        }

        buffer.retain(|e| e.recorded_at >= timestamp_ms);
        Ok(before_len - buffer.len())
    }

    fn clone_store(&self) -> Box<dyn ReplayStore> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustapi_core::replay::{RecordedRequest, RecordedResponse, ReplayMeta};

    fn make_entry(method: &str, path: &str, status: u16) -> ReplayEntry {
        ReplayEntry::new(
            RecordedRequest::new(method, path, path),
            RecordedResponse::new(status),
            ReplayMeta::new(),
        )
    }

    #[tokio::test]
    async fn test_store_and_get() {
        let store = InMemoryReplayStore::new(10);
        let entry = make_entry("GET", "/users", 200);
        let id = entry.id.clone();

        store.store(entry).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_list() {
        let store = InMemoryReplayStore::new(10);
        store.store(make_entry("GET", "/users", 200)).await.unwrap();
        store
            .store(make_entry("POST", "/users", 201))
            .await
            .unwrap();
        store
            .store(make_entry("GET", "/items", 404))
            .await
            .unwrap();

        let all = store.list(&ReplayQuery::new()).await.unwrap();
        assert_eq!(all.len(), 3);

        let filtered = store
            .list(&ReplayQuery::new().method("GET"))
            .await
            .unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[tokio::test]
    async fn test_ring_buffer_eviction() {
        let store = InMemoryReplayStore::new(2);

        let e1 = make_entry("GET", "/a", 200);
        let id1 = e1.id.clone();
        store.store(e1).await.unwrap();
        store.store(make_entry("GET", "/b", 200)).await.unwrap();
        store.store(make_entry("GET", "/c", 200)).await.unwrap(); // evicts e1

        assert_eq!(store.count().await.unwrap(), 2);
        assert!(store.get(&id1).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let store = InMemoryReplayStore::new(10);
        let entry = make_entry("GET", "/users", 200);
        let id = entry.id.clone();

        store.store(entry).await.unwrap();
        assert!(store.delete(&id).await.unwrap());
        assert!(store.get(&id).await.unwrap().is_none());
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_clear() {
        let store = InMemoryReplayStore::new(10);
        store.store(make_entry("GET", "/a", 200)).await.unwrap();
        store.store(make_entry("GET", "/b", 200)).await.unwrap();

        store.clear().await.unwrap();
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_delete_before() {
        let store = InMemoryReplayStore::new(10);

        let mut e1 = make_entry("GET", "/a", 200);
        e1.recorded_at = 1000;
        let mut e2 = make_entry("GET", "/b", 200);
        e2.recorded_at = 2000;
        let mut e3 = make_entry("GET", "/c", 200);
        e3.recorded_at = 3000;

        store.store(e1).await.unwrap();
        store.store(e2).await.unwrap();
        store.store(e3).await.unwrap();

        let deleted = store.delete_before(2500).await.unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(store.count().await.unwrap(), 1);
    }
}
