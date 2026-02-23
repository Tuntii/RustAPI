//! Integration tests for rustapi-memory
//!
//! Tests InMemoryStore, ConversationMemory, MemoryQuery filtering,
//! TTL expiry, capacity limits, and session management.

use rustapi_memory::backend::InMemoryStore;
use rustapi_memory::*;
use serde_json::json;
use std::sync::Arc;

// ===========================================================================
// InMemoryStore: advanced filtering & pagination
// ===========================================================================

#[tokio::test]
async fn test_in_memory_store_key_prefix_filter() {
    let store = InMemoryStore::new();

    store.store(MemoryEntry::new("user:1:name", json!("Alice")).with_namespace("users")).await.unwrap();
    store.store(MemoryEntry::new("user:1:email", json!("alice@example.com")).with_namespace("users")).await.unwrap();
    store.store(MemoryEntry::new("user:2:name", json!("Bob")).with_namespace("users")).await.unwrap();
    store.store(MemoryEntry::new("config:theme", json!("dark"))).await.unwrap();

    let query = MemoryQuery::new()
        .with_namespace("users")
        .with_key_prefix("user:1:");

    let results = store.list(&query).await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|e| e.key.starts_with("user:1:")));
}

#[tokio::test]
async fn test_in_memory_store_pagination() {
    let store = InMemoryStore::new();

    for i in 0..10 {
        let entry = MemoryEntry::new(format!("item:{i:02}"), json!(i)).with_namespace("items");
        store.store(entry).await.unwrap();
    }

    // Page 1: items 0-4
    let query = MemoryQuery {
        namespace: Some("items".into()),
        limit: 5,
        offset: 0,
        newest_first: false,
        ..Default::default()
    };
    let page1 = store.list(&query).await.unwrap();
    assert_eq!(page1.len(), 5);

    // Page 2: items 5-9
    let query2 = MemoryQuery {
        namespace: Some("items".into()),
        limit: 5,
        offset: 5,
        newest_first: false,
        ..Default::default()
    };
    let page2 = store.list(&query2).await.unwrap();
    assert_eq!(page2.len(), 5);

    // Pages don't overlap.
    let keys1: Vec<_> = page1.iter().map(|e| &e.key).collect();
    let keys2: Vec<_> = page2.iter().map(|e| &e.key).collect();
    assert!(keys1.iter().all(|k| !keys2.contains(k)));
}

#[tokio::test]
async fn test_in_memory_store_clear_by_namespace() {
    let store = InMemoryStore::new();

    store.store(MemoryEntry::new("a", json!(1)).with_namespace("ns1")).await.unwrap();
    store.store(MemoryEntry::new("b", json!(2)).with_namespace("ns1")).await.unwrap();
    store.store(MemoryEntry::new("c", json!(3)).with_namespace("ns2")).await.unwrap();

    // Clear only ns1.
    store.clear(Some("ns1")).await.unwrap();

    assert_eq!(store.count(Some("ns1")).await.unwrap(), 0);
    assert_eq!(store.count(Some("ns2")).await.unwrap(), 1);
    assert!(store.get("c").await.unwrap().is_some());
}

#[tokio::test]
async fn test_in_memory_store_clear_all() {
    let store = InMemoryStore::new();

    store.store(MemoryEntry::new("a", json!(1))).await.unwrap();
    store.store(MemoryEntry::new("b", json!(2))).await.unwrap();

    store.clear(None).await.unwrap();
    assert_eq!(store.count(None).await.unwrap(), 0);
}

#[tokio::test]
async fn test_in_memory_store_upsert() {
    let store = InMemoryStore::new();

    store.store(MemoryEntry::new("key", json!(1))).await.unwrap();
    assert_eq!(store.get("key").await.unwrap().unwrap().value, json!(1));

    // Overwrite with same key.
    store.store(MemoryEntry::new("key", json!(42))).await.unwrap();
    assert_eq!(store.get("key").await.unwrap().unwrap().value, json!(42));

    // Count should still be 1.
    assert_eq!(store.count(None).await.unwrap(), 1);
}

#[tokio::test]
async fn test_in_memory_store_capacity_allows_upsert() {
    let store = InMemoryStore::with_capacity(2);

    store.store(MemoryEntry::new("a", json!(1))).await.unwrap();
    store.store(MemoryEntry::new("b", json!(2))).await.unwrap();

    // At capacity, new key should fail.
    let result = store.store(MemoryEntry::new("c", json!(3))).await;
    assert!(result.is_err());

    // But updating existing key should succeed (upsert).
    store.store(MemoryEntry::new("a", json!(100))).await.unwrap();
    assert_eq!(store.get("a").await.unwrap().unwrap().value, json!(100));
}

#[tokio::test]
async fn test_in_memory_store_evict_expired() {
    let store = InMemoryStore::new();

    // Store an entry with 0-second TTL (immediately expired).
    let entry = MemoryEntry::new("ephemeral", json!("gone")).with_ttl(0);
    store.store(entry).await.unwrap();

    // ttl=0 means it's expired after at least 1 second. Give it a moment.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    // Evict expired entries.
    store.evict_expired();

    // Should be gone after eviction.
    assert!(store.get("ephemeral").await.unwrap().is_none());
    assert_eq!(store.count(None).await.unwrap(), 0);
}

#[tokio::test]
async fn test_in_memory_store_lazy_eviction_on_get() {
    let store = InMemoryStore::new();

    let entry = MemoryEntry::new("lazy", json!("temp")).with_ttl(0);
    store.store(entry).await.unwrap();

    // Wait for expiry.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    // get() should return None and lazily evict.
    assert!(store.get("lazy").await.unwrap().is_none());
}

// ===========================================================================
// MemoryEntry builder patterns
// ===========================================================================

#[test]
fn test_memory_entry_builder() {
    let entry = MemoryEntry::new("key", json!({"data": true}))
        .with_namespace("test")
        .with_embedding(vec![0.1, 0.2, 0.3])
        .with_ttl(3600)
        .with_metadata("source", json!("test"));

    assert_eq!(entry.key, "key");
    assert_eq!(entry.namespace.as_deref(), Some("test"));
    assert_eq!(entry.embedding.as_ref().unwrap().len(), 3);
    assert_eq!(entry.ttl_secs, Some(3600));
    assert_eq!(entry.metadata.get("source"), Some(&json!("test")));
}

#[test]
fn test_memory_entry_not_expired_without_ttl() {
    let entry = MemoryEntry::new("forever", json!(null));
    assert!(!entry.is_expired());
}

#[test]
fn test_memory_entry_serialization() {
    let entry = MemoryEntry::new("ser", json!({"x": 1})).with_namespace("ns");
    let json_str = serde_json::to_string(&entry).unwrap();
    let deserialized: MemoryEntry = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.key, "ser");
    assert_eq!(deserialized.namespace, Some("ns".into()));
}

// ===========================================================================
// MemoryQuery::matches
// ===========================================================================

#[test]
fn test_memory_query_matches_namespace() {
    let query = MemoryQuery::new().with_namespace("alpha");
    let entry_a = MemoryEntry::new("a", json!(1)).with_namespace("alpha");
    let entry_b = MemoryEntry::new("b", json!(2)).with_namespace("beta");

    assert!(query.matches(&entry_a));
    assert!(!query.matches(&entry_b));
}

#[test]
fn test_memory_query_matches_key_prefix() {
    let query = MemoryQuery::new().with_key_prefix("user:");
    let entry_yes = MemoryEntry::new("user:42", json!("Alice"));
    let entry_no = MemoryEntry::new("config:theme", json!("dark"));

    assert!(query.matches(&entry_yes));
    assert!(!query.matches(&entry_no));
}

#[test]
fn test_memory_query_matches_all_empty() {
    let query = MemoryQuery::new();
    let entry = MemoryEntry::new("anything", json!(null));
    assert!(query.matches(&entry));
}

// ===========================================================================
// ConversationMemory
// ===========================================================================

#[tokio::test]
async fn test_conversation_memory_clear_session() {
    let store = Arc::new(InMemoryStore::new());
    let conv = ConversationMemory::new(store);

    conv.add_turn("s1", Turn::user("Hello")).await.unwrap();
    conv.add_turn("s1", Turn::assistant("Hi")).await.unwrap();

    assert_eq!(conv.get_turns("s1").await.unwrap().len(), 2);

    conv.clear_session("s1").await.unwrap();
    assert_eq!(conv.get_turns("s1").await.unwrap().len(), 0);
}

#[tokio::test]
async fn test_conversation_memory_get_recent_turns() {
    let store = Arc::new(InMemoryStore::new());
    let conv = ConversationMemory::new(store);

    for i in 0..5 {
        conv.add_turn("s1", Turn::user(format!("msg {i}")))
            .await
            .unwrap();
    }

    let recent = conv.get_recent_turns("s1", 2).await.unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].content, "msg 3");
    assert_eq!(recent[1].content, "msg 4");
}

#[tokio::test]
async fn test_conversation_memory_list_sessions() {
    let store = Arc::new(InMemoryStore::new());
    let conv = ConversationMemory::new(store);

    conv.add_turn("session-a", Turn::user("Hello A")).await.unwrap();
    conv.add_turn("session-b", Turn::user("Hello B")).await.unwrap();
    conv.add_turn("session-c", Turn::user("Hello C")).await.unwrap();

    let sessions = conv.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 3);
    assert!(sessions.contains(&"session-a".to_string()));
    assert!(sessions.contains(&"session-b".to_string()));
    assert!(sessions.contains(&"session-c".to_string()));
}

#[tokio::test]
async fn test_conversation_memory_tool_turn() {
    let store = Arc::new(InMemoryStore::new());
    let conv = ConversationMemory::new(store);

    conv.add_turn("s1", Turn::system("You are a helpful assistant")).await.unwrap();
    conv.add_turn("s1", Turn::user("Search for Rust tutorials")).await.unwrap();
    conv.add_turn("s1", Turn::tool("Found 42 results", "call-123")).await.unwrap();
    conv.add_turn("s1", Turn::assistant("I found 42 results!")).await.unwrap();

    let turns = conv.get_turns("s1").await.unwrap();
    assert_eq!(turns.len(), 4);
    assert_eq!(turns[0].role, Role::System);
    assert_eq!(turns[2].role, Role::Tool);
    assert_eq!(turns[2].tool_call_id, Some("call-123".to_string()));
}

#[tokio::test]
async fn test_conversation_memory_empty_session() {
    let store = Arc::new(InMemoryStore::new());
    let conv = ConversationMemory::new(store);

    // Getting turns for a non-existent session should return empty vec.
    let turns = conv.get_turns("nonexistent").await.unwrap();
    assert!(turns.is_empty());
}

// ===========================================================================
// clone_store trait method
// ===========================================================================

#[tokio::test]
async fn test_memory_store_clone_store() {
    let store = InMemoryStore::new();
    store.store(MemoryEntry::new("k1", json!("v1"))).await.unwrap();

    let cloned: Box<dyn MemoryStore> = store.clone_store();
    // Cloned store shares same data (Arc).
    let val = cloned.get("k1").await.unwrap();
    assert!(val.is_some());
    assert_eq!(val.unwrap().value, json!("v1"));
}
