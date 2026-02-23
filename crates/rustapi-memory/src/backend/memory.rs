use crate::{MemoryEntry, MemoryError, MemoryQuery, MemoryStore};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory [`MemoryStore`] implementation backed by `HashMap`.
///
/// Suitable for development, testing, and single-instance deployments.
/// Entries with TTL are lazily evicted on access.
#[derive(Debug, Clone)]
pub struct InMemoryStore {
    entries: Arc<RwLock<HashMap<String, MemoryEntry>>>,
    max_capacity: Option<usize>,
}

impl InMemoryStore {
    /// Create a new in-memory store with no capacity limit.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_capacity: None,
        }
    }

    /// Create a new in-memory store with a maximum capacity.
    pub fn with_capacity(max: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::with_capacity(max))),
            max_capacity: Some(max),
        }
    }

    /// Remove expired entries (garbage collection).
    pub fn evict_expired(&self) {
        if let Ok(mut map) = self.entries.write() {
            map.retain(|_, entry| !entry.is_expired());
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryStore for InMemoryStore {
    async fn store(&self, entry: MemoryEntry) -> Result<(), MemoryError> {
        let mut map = self
            .entries
            .write()
            .map_err(|e| MemoryError::internal(e.to_string()))?;

        // Check capacity (only for new inserts).
        if let Some(max) = self.max_capacity {
            if !map.contains_key(&entry.key) && map.len() >= max {
                return Err(MemoryError::CapacityExceeded);
            }
        }

        map.insert(entry.key.clone(), entry);
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let map = self
            .entries
            .read()
            .map_err(|e| MemoryError::internal(e.to_string()))?;

        match map.get(key) {
            Some(entry) if entry.is_expired() => {
                drop(map);
                // Lazy eviction.
                if let Ok(mut wmap) = self.entries.write() {
                    wmap.remove(key);
                }
                Ok(None)
            }
            Some(entry) => Ok(Some(entry.clone())),
            None => Ok(None),
        }
    }

    async fn list(&self, query: &MemoryQuery) -> Result<Vec<MemoryEntry>, MemoryError> {
        let map = self
            .entries
            .read()
            .map_err(|e| MemoryError::internal(e.to_string()))?;

        let mut results: Vec<MemoryEntry> = map
            .values()
            .filter(|e| !e.is_expired() && query.matches(e))
            .cloned()
            .collect();

        // Sort.
        if query.newest_first {
            results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        } else {
            results.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        }

        // Paginate.
        let start = query.offset.min(results.len());
        let end = (start + query.limit).min(results.len());
        Ok(results[start..end].to_vec())
    }

    async fn delete(&self, key: &str) -> Result<bool, MemoryError> {
        let mut map = self
            .entries
            .write()
            .map_err(|e| MemoryError::internal(e.to_string()))?;
        Ok(map.remove(key).is_some())
    }

    async fn count(&self, namespace: Option<&str>) -> Result<usize, MemoryError> {
        let map = self
            .entries
            .read()
            .map_err(|e| MemoryError::internal(e.to_string()))?;

        let count = match namespace {
            Some(ns) => map
                .values()
                .filter(|e| !e.is_expired() && e.namespace.as_deref() == Some(ns))
                .count(),
            None => map.values().filter(|e| !e.is_expired()).count(),
        };
        Ok(count)
    }

    async fn clear(&self, namespace: Option<&str>) -> Result<(), MemoryError> {
        let mut map = self
            .entries
            .write()
            .map_err(|e| MemoryError::internal(e.to_string()))?;

        match namespace {
            Some(ns) => {
                map.retain(|_, e| e.namespace.as_deref() != Some(ns));
            }
            None => map.clear(),
        }
        Ok(())
    }

    fn clone_store(&self) -> Box<dyn MemoryStore> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_store_crud() {
        let store = InMemoryStore::new();

        // Store
        let entry = MemoryEntry::new("k1", serde_json::json!({"msg": "hello"}))
            .with_namespace("test");
        store.store(entry).await.unwrap();

        // Get
        let retrieved = store.get("k1").await.unwrap().unwrap();
        assert_eq!(retrieved.key, "k1");
        assert_eq!(retrieved.value, serde_json::json!({"msg": "hello"}));

        // Count
        assert_eq!(store.count(Some("test")).await.unwrap(), 1);
        assert_eq!(store.count(Some("other")).await.unwrap(), 0);

        // Delete
        assert!(store.delete("k1").await.unwrap());
        assert!(!store.delete("k1").await.unwrap());
        assert!(store.get("k1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_in_memory_store_list() {
        let store = InMemoryStore::new();

        for i in 0..5 {
            let entry = MemoryEntry::new(format!("item:{i}"), serde_json::json!(i))
                .with_namespace("ns1");
            store.store(entry).await.unwrap();
        }

        let query = MemoryQuery::new().with_namespace("ns1").with_limit(3);
        let results = store.list(&query).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_in_memory_store_capacity() {
        let store = InMemoryStore::with_capacity(2);

        store
            .store(MemoryEntry::new("a", serde_json::json!(1)))
            .await
            .unwrap();
        store
            .store(MemoryEntry::new("b", serde_json::json!(2)))
            .await
            .unwrap();

        let result = store
            .store(MemoryEntry::new("c", serde_json::json!(3)))
            .await;
        assert!(matches!(result, Err(MemoryError::CapacityExceeded)));

        // Updating existing key should work.
        store
            .store(MemoryEntry::new("a", serde_json::json!(10)))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_in_memory_store_clear_namespace() {
        let store = InMemoryStore::new();

        store
            .store(MemoryEntry::new("a", serde_json::json!(1)).with_namespace("ns1"))
            .await
            .unwrap();
        store
            .store(MemoryEntry::new("b", serde_json::json!(2)).with_namespace("ns2"))
            .await
            .unwrap();

        store.clear(Some("ns1")).await.unwrap();
        assert_eq!(store.count(None).await.unwrap(), 1);
    }
}
