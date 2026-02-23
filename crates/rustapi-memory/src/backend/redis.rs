//! Redis-backed [`MemoryStore`](crate::MemoryStore) implementation.
//!
//! Requires the `redis` feature flag. Entries are stored as JSON strings
//! with optional Redis `EXPIRE` for TTL support.
//!
//! # Key Layout
//!
//! | Redis Key | Purpose |
//! |-----------|---------|
//! | `{prefix}:entry:{key}` | Stores the JSON-serialized `MemoryEntry` |
//! | `{prefix}:ns:{namespace}` | Redis SET tracking keys within a namespace |
//! | `{prefix}:all` | Redis SET tracking all keys |
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_memory::backend::RedisStore;
//!
//! let store = RedisStore::new("redis://127.0.0.1:6379", "myapp").await?;
//! ```

use crate::{MemoryEntry, MemoryError, MemoryQuery, MemoryStore};
use async_trait::async_trait;
use redis::AsyncCommands;
use std::sync::Arc;

/// Redis-backed memory store.
///
/// Uses JSON serialization for entries and Redis native EXPIRE for TTL.
/// Namespace tracking is handled through Redis SETs for efficient enumeration.
#[derive(Clone)]
pub struct RedisStore {
    client: redis::Client,
    prefix: Arc<String>,
}

impl std::fmt::Debug for RedisStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisStore")
            .field("prefix", &self.prefix)
            .finish_non_exhaustive()
    }
}

impl RedisStore {
    /// Create a new Redis store.
    ///
    /// # Arguments
    /// * `url` – Redis connection URL (e.g. `redis://127.0.0.1:6379`)
    /// * `prefix` – Key prefix to namespace all keys in Redis (e.g. `"rustapi"`)
    pub async fn new(
        url: impl AsRef<str>,
        prefix: impl Into<String>,
    ) -> Result<Self, MemoryError> {
        let client = redis::Client::open(url.as_ref())
            .map_err(|e| MemoryError::backend(format!("Redis connection error: {e}")))?;

        // Verify connectivity.
        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| MemoryError::backend(format!("Redis connection error: {e}")))?;

        // Ping to verify.
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .map_err(|e| MemoryError::backend(format!("Redis ping failed: {e}")))?;

        Ok(Self {
            client,
            prefix: Arc::new(prefix.into()),
        })
    }

    /// Create a new Redis store from an already-constructed `redis::Client`.
    ///
    /// Does **not** verify connectivity; call [`Self::ping`] if desired.
    pub fn from_client(client: redis::Client, prefix: impl Into<String>) -> Self {
        Self {
            client,
            prefix: Arc::new(prefix.into()),
        }
    }

    /// Verify the connection is alive.
    pub async fn ping(&self) -> Result<(), MemoryError> {
        let mut conn = self.conn().await?;
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .map_err(|e| MemoryError::backend(format!("Redis ping failed: {e}")))?;
        Ok(())
    }

    // -- internal helpers ----------------------------------------------------

    async fn conn(&self) -> Result<redis::aio::MultiplexedConnection, MemoryError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| MemoryError::backend(format!("Redis connection error: {e}")))
    }

    /// Build the Redis key for an entry.
    fn entry_key(&self, key: &str) -> String {
        format!("{}:entry:{}", self.prefix, key)
    }

    /// Build the Redis key for the "all keys" set.
    fn all_set_key(&self) -> String {
        format!("{}:all", self.prefix)
    }

    /// Build the Redis key for a namespace set.
    fn ns_set_key(&self, ns: &str) -> String {
        format!("{}:ns:{}", self.prefix, ns)
    }

    /// Serialize a `MemoryEntry` to JSON string.
    fn serialize(entry: &MemoryEntry) -> Result<String, MemoryError> {
        serde_json::to_string(entry).map_err(|e| MemoryError::serialization(e.to_string()))
    }

    /// Deserialize a JSON string into a `MemoryEntry`.
    fn deserialize(data: &str) -> Result<MemoryEntry, MemoryError> {
        serde_json::from_str(data).map_err(|e| MemoryError::serialization(e.to_string()))
    }
}

#[async_trait]
impl MemoryStore for RedisStore {
    async fn store(&self, entry: MemoryEntry) -> Result<(), MemoryError> {
        let mut conn = self.conn().await?;
        let rkey = self.entry_key(&entry.key);
        let json = Self::serialize(&entry)?;

        // Store JSON value.
        conn.set::<_, _, ()>(&rkey, &json)
            .await
            .map_err(|e| MemoryError::backend(format!("Redis SET error: {e}")))?;

        // Set TTL if provided.
        if let Some(ttl) = entry.ttl_secs {
            conn.expire::<_, ()>(&rkey, ttl as i64)
                .await
                .map_err(|e| MemoryError::backend(format!("Redis EXPIRE error: {e}")))?;
        }

        // Track key in the "all" set.
        conn.sadd::<_, _, ()>(&self.all_set_key(), &entry.key)
            .await
            .map_err(|e| MemoryError::backend(format!("Redis SADD error: {e}")))?;

        // Track key in namespace set if applicable.
        if let Some(ref ns) = entry.namespace {
            conn.sadd::<_, _, ()>(&self.ns_set_key(ns), &entry.key)
                .await
                .map_err(|e| MemoryError::backend(format!("Redis SADD ns error: {e}")))?;
        }

        tracing::debug!(key = %entry.key, "Stored memory entry in Redis");
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let mut conn = self.conn().await?;
        let rkey = self.entry_key(key);

        let data: Option<String> = conn
            .get(&rkey)
            .await
            .map_err(|e| MemoryError::backend(format!("Redis GET error: {e}")))?;

        match data {
            Some(json) => {
                let entry = Self::deserialize(&json)?;
                // Double-check logical TTL (Redis EXPIRE handles physical TTL,
                // but entry.is_expired() uses the chrono-based check).
                if entry.is_expired() {
                    // Clean up stale tracking sets.
                    self.delete(key).await?;
                    Ok(None)
                } else {
                    Ok(Some(entry))
                }
            }
            None => {
                // Entry gone from Redis (expired by Redis or deleted).
                // Clean up tracking sets just in case.
                conn.srem::<_, _, ()>(&self.all_set_key(), key)
                    .await
                    .ok();
                Ok(None)
            }
        }
    }

    async fn list(&self, query: &MemoryQuery) -> Result<Vec<MemoryEntry>, MemoryError> {
        let mut conn = self.conn().await?;

        // Determine which set to scan.
        let members: Vec<String> = if let Some(ref ns) = query.namespace {
            conn.smembers(self.ns_set_key(ns))
                .await
                .map_err(|e| MemoryError::backend(format!("Redis SMEMBERS error: {e}")))?
        } else {
            conn.smembers(self.all_set_key())
                .await
                .map_err(|e| MemoryError::backend(format!("Redis SMEMBERS error: {e}")))?
        };

        // Fetch each entry and apply filters.
        let mut results = Vec::new();
        let mut stale_keys = Vec::new();

        for member_key in &members {
            let rkey = self.entry_key(member_key);
            let data: Option<String> = conn
                .get(&rkey)
                .await
                .map_err(|e| MemoryError::backend(format!("Redis GET error: {e}")))?;

            match data {
                Some(json) => {
                    let entry = Self::deserialize(&json)?;
                    if entry.is_expired() {
                        stale_keys.push(member_key.clone());
                    } else if query.matches(&entry) {
                        results.push(entry);
                    }
                }
                None => {
                    // Key expired in Redis but still in tracking set.
                    stale_keys.push(member_key.clone());
                }
            }
        }

        // Async cleanup of stale tracking entries.
        if !stale_keys.is_empty() {
            let all_key = self.all_set_key();
            for sk in &stale_keys {
                conn.srem::<_, _, ()>(&all_key, sk).await.ok();
            }
            // Also remove from namespace set if filtered by namespace.
            if let Some(ref ns) = query.namespace {
                let ns_key = self.ns_set_key(ns);
                for sk in &stale_keys {
                    conn.srem::<_, _, ()>(&ns_key, sk).await.ok();
                }
            }
        }

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
        let mut conn = self.conn().await?;
        let rkey = self.entry_key(key);

        // Read the entry first to know its namespace.
        let data: Option<String> = conn
            .get(&rkey)
            .await
            .map_err(|e| MemoryError::backend(format!("Redis GET error: {e}")))?;

        let deleted: i64 = conn
            .del(&rkey)
            .await
            .map_err(|e| MemoryError::backend(format!("Redis DEL error: {e}")))?;

        // Remove from tracking sets.
        conn.srem::<_, _, ()>(&self.all_set_key(), key)
            .await
            .ok();

        if let Some(json) = data {
            if let Ok(entry) = Self::deserialize(&json) {
                if let Some(ref ns) = entry.namespace {
                    conn.srem::<_, _, ()>(&self.ns_set_key(ns), key)
                        .await
                        .ok();
                }
            }
        }

        Ok(deleted > 0)
    }

    async fn count(&self, namespace: Option<&str>) -> Result<usize, MemoryError> {
        let mut conn = self.conn().await?;

        let set_key = match namespace {
            Some(ns) => self.ns_set_key(ns),
            None => self.all_set_key(),
        };

        let count: usize = conn
            .scard(&set_key)
            .await
            .map_err(|e| MemoryError::backend(format!("Redis SCARD error: {e}")))?;

        Ok(count)
    }

    async fn clear(&self, namespace: Option<&str>) -> Result<(), MemoryError> {
        let mut conn = self.conn().await?;

        match namespace {
            Some(ns) => {
                // Get all keys in the namespace.
                let members: Vec<String> = conn
                    .smembers(self.ns_set_key(ns))
                    .await
                    .map_err(|e| MemoryError::backend(format!("Redis SMEMBERS error: {e}")))?;

                // Delete each entry and remove from "all" set.
                for member_key in &members {
                    let rkey = self.entry_key(member_key);
                    conn.del::<_, ()>(&rkey).await.ok();
                    conn.srem::<_, _, ()>(&self.all_set_key(), member_key)
                        .await
                        .ok();
                }

                // Delete the namespace set itself.
                conn.del::<_, ()>(&self.ns_set_key(ns)).await.ok();

                tracing::debug!(namespace = %ns, count = members.len(), "Cleared namespace in Redis");
            }
            None => {
                // Get all keys.
                let members: Vec<String> = conn
                    .smembers(self.all_set_key())
                    .await
                    .map_err(|e| MemoryError::backend(format!("Redis SMEMBERS error: {e}")))?;

                // Delete each entry.
                for member_key in &members {
                    let rkey = self.entry_key(member_key);
                    conn.del::<_, ()>(&rkey).await.ok();
                }

                // Also clean up any namespace sets – use SCAN for prefix.
                let ns_prefix = format!("{}:ns:", self.prefix);
                let mut cursor: u64 = 0;
                loop {
                    let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                        .arg(cursor)
                        .arg("MATCH")
                        .arg(format!("{ns_prefix}*"))
                        .arg("COUNT")
                        .arg(100)
                        .query_async(&mut conn)
                        .await
                        .map_err(|e| {
                            MemoryError::backend(format!("Redis SCAN error: {e}"))
                        })?;

                    for k in &keys {
                        conn.del::<_, ()>(k).await.ok();
                    }

                    cursor = next_cursor;
                    if cursor == 0 {
                        break;
                    }
                }

                // Delete the "all" set.
                conn.del::<_, ()>(&self.all_set_key()).await.ok();

                tracing::debug!(count = members.len(), "Cleared all entries in Redis");
            }
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

    #[test]
    fn test_key_helpers() {
        let store = RedisStore::from_client(
            redis::Client::open("redis://localhost").unwrap(),
            "myapp",
        );
        assert_eq!(store.entry_key("user:42"), "myapp:entry:user:42");
        assert_eq!(store.all_set_key(), "myapp:all");
        assert_eq!(store.ns_set_key("session"), "myapp:ns:session");
    }

    #[test]
    fn test_serialize_deserialize() {
        let entry = MemoryEntry::new("k1", serde_json::json!({"hello": "world"}))
            .with_namespace("test")
            .with_ttl(600)
            .with_metadata("role", serde_json::json!("admin"));

        let json = RedisStore::serialize(&entry).unwrap();
        let back = RedisStore::deserialize(&json).unwrap();

        assert_eq!(back.key, "k1");
        assert_eq!(back.namespace, Some("test".into()));
        assert_eq!(back.ttl_secs, Some(600));
        assert_eq!(
            back.metadata.get("role"),
            Some(&serde_json::json!("admin"))
        );
    }

    #[test]
    fn test_from_client_does_not_connect() {
        // from_client should not attempt any IO.
        let _store = RedisStore::from_client(
            redis::Client::open("redis://nonexistent:9999").unwrap(),
            "test",
        );
    }

    #[tokio::test]
    async fn test_new_fails_on_bad_url() {
        let result = RedisStore::new("not-a-valid-url", "test").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            MemoryError::BackendError { .. } => {}
            other => panic!("Expected BackendError, got: {other:?}"),
        }
    }

    #[test]
    fn test_debug_impl() {
        let store = RedisStore::from_client(
            redis::Client::open("redis://localhost").unwrap(),
            "debug-test",
        );
        let debug = format!("{store:?}");
        assert!(debug.contains("RedisStore"));
        assert!(debug.contains("debug-test"));
    }

    #[test]
    fn test_clone() {
        let store = RedisStore::from_client(
            redis::Client::open("redis://localhost").unwrap(),
            "clone-test",
        );
        let cloned = store.clone();
        assert_eq!(cloned.entry_key("x"), "clone-test:entry:x");
    }

    #[test]
    fn test_clone_store_trait() {
        let store = RedisStore::from_client(
            redis::Client::open("redis://localhost").unwrap(),
            "trait-test",
        );
        let _boxed: Box<dyn MemoryStore> = store.clone_store();
    }
}
