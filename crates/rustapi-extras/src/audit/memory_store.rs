//! In-memory audit store implementation

use super::event::AuditEvent;
use super::query::AuditQuery;
use super::store::{AuditError, AuditResult, AuditStore};
use std::sync::RwLock;

/// Configuration for in-memory audit store.
#[derive(Debug, Clone)]
pub struct InMemoryAuditStoreConfig {
    /// Maximum number of events to store.
    pub max_events: usize,
    /// Whether to remove oldest events when full (ring buffer behavior).
    pub evict_oldest: bool,
}

impl Default for InMemoryAuditStoreConfig {
    fn default() -> Self {
        Self {
            max_events: 10000,
            evict_oldest: true,
        }
    }
}

/// In-memory audit store (for development/testing).
pub struct InMemoryAuditStore {
    events: RwLock<Vec<AuditEvent>>,
    config: InMemoryAuditStoreConfig,
}

impl InMemoryAuditStore {
    /// Create a new in-memory audit store with default configuration.
    pub fn new() -> Self {
        Self::with_config(InMemoryAuditStoreConfig::default())
    }

    /// Create a new in-memory audit store with custom configuration.
    pub fn with_config(config: InMemoryAuditStoreConfig) -> Self {
        Self {
            events: RwLock::new(Vec::with_capacity(config.max_events.min(1000))),
            config,
        }
    }

    /// Create a bounded store with the specified maximum events.
    pub fn bounded(max_events: usize) -> Self {
        Self::with_config(InMemoryAuditStoreConfig {
            max_events,
            evict_oldest: true,
        })
    }
}

impl Default for InMemoryAuditStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditStore for InMemoryAuditStore {
    fn log(&self, event: AuditEvent) -> AuditResult<()> {
        let mut events = self
            .events
            .write()
            .map_err(|e| AuditError::WriteError(format!("Failed to acquire lock: {}", e)))?;

        // Check capacity
        if events.len() >= self.config.max_events {
            if self.config.evict_oldest {
                events.remove(0);
            } else {
                return Err(AuditError::StorageFull);
            }
        }

        events.push(event);
        Ok(())
    }

    fn get(&self, id: &str) -> AuditResult<Option<AuditEvent>> {
        let events = self
            .events
            .read()
            .map_err(|e| AuditError::ReadError(format!("Failed to acquire lock: {}", e)))?;

        Ok(events.iter().find(|e| e.id == id).cloned())
    }

    fn execute_query(&self, query: &AuditQuery) -> AuditResult<Vec<AuditEvent>> {
        let events = self
            .events
            .read()
            .map_err(|e| AuditError::ReadError(format!("Failed to acquire lock: {}", e)))?;

        let mut results: Vec<AuditEvent> = events
            .iter()
            .filter(|e| query.matches(e))
            .cloned()
            .collect();

        // Sort by timestamp
        if query.newest_first {
            results.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        } else {
            results.sort_by_key(|a| a.timestamp);
        }

        // Apply offset and limit
        let offset = query.offset.unwrap_or(0);
        let results: Vec<AuditEvent> = results.into_iter().skip(offset).collect();

        let results = if let Some(limit) = query.limit {
            results.into_iter().take(limit).collect()
        } else {
            results
        };

        Ok(results)
    }

    fn count(&self, query: &AuditQuery) -> AuditResult<usize> {
        let events = self
            .events
            .read()
            .map_err(|e| AuditError::ReadError(format!("Failed to acquire lock: {}", e)))?;

        Ok(events.iter().filter(|e| query.matches(e)).count())
    }

    fn total_count(&self) -> AuditResult<usize> {
        let events = self
            .events
            .read()
            .map_err(|e| AuditError::ReadError(format!("Failed to acquire lock: {}", e)))?;

        Ok(events.len())
    }

    fn clear(&self) -> AuditResult<()> {
        let mut events = self
            .events
            .write()
            .map_err(|e| AuditError::WriteError(format!("Failed to acquire lock: {}", e)))?;

        events.clear();
        Ok(())
    }

    fn flush(&self) -> AuditResult<()> {
        // No-op for in-memory store
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditAction, ComplianceInfo};

    #[test]
    fn test_in_memory_store_log_and_get() {
        let store = InMemoryAuditStore::new();

        let event = AuditEvent::new(AuditAction::Create)
            .resource("users", "user-123")
            .actor("admin");

        let id = event.id.clone();
        store.log(event).unwrap();

        let retrieved = store.get(&id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().actor_id, Some("admin".to_string()));
    }

    #[test]
    fn test_in_memory_store_query() {
        let store = InMemoryAuditStore::new();

        // Log multiple events
        store
            .log(AuditEvent::new(AuditAction::Create).actor("alice"))
            .unwrap();
        store
            .log(AuditEvent::new(AuditAction::Read).actor("bob"))
            .unwrap();
        store
            .log(AuditEvent::new(AuditAction::Create).actor("alice"))
            .unwrap();

        // Query by actor
        let results = store.query().actor("alice").execute().unwrap();
        assert_eq!(results.len(), 2);

        // Query by action
        let results = store.query().action(AuditAction::Read).execute().unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_in_memory_store_bounded() {
        let store = InMemoryAuditStore::bounded(3);

        store
            .log(AuditEvent::new(AuditAction::Create).actor("a"))
            .unwrap();
        store
            .log(AuditEvent::new(AuditAction::Create).actor("b"))
            .unwrap();
        store
            .log(AuditEvent::new(AuditAction::Create).actor("c"))
            .unwrap();
        store
            .log(AuditEvent::new(AuditAction::Create).actor("d"))
            .unwrap();

        // Should only have 3 events (oldest evicted)
        assert_eq!(store.total_count().unwrap(), 3);

        // First event should be gone
        let results = store.query().actor("a").execute().unwrap();
        assert_eq!(results.len(), 0);

        // Latest should be there
        let results = store.query().actor("d").execute().unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_in_memory_store_personal_data_filter() {
        let store = InMemoryAuditStore::new();

        let compliance = ComplianceInfo::new()
            .personal_data(true)
            .data_subject("user-456");

        store
            .log(AuditEvent::new(AuditAction::Update).compliance(compliance))
            .unwrap();
        store.log(AuditEvent::new(AuditAction::Read)).unwrap();

        let results = store.query().personal_data(true).execute().unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_in_memory_store_pagination() {
        let store = InMemoryAuditStore::new();

        for i in 0..10 {
            store
                .log(AuditEvent::new(AuditAction::Read).meta("index", i.to_string()))
                .unwrap();
        }

        // First page
        let page1 = store.query().limit(3).offset(0).execute().unwrap();
        assert_eq!(page1.len(), 3);

        // Second page
        let page2 = store.query().limit(3).offset(3).execute().unwrap();
        assert_eq!(page2.len(), 3);

        // Verify they're different
        assert_ne!(page1[0].id, page2[0].id);
    }
}
