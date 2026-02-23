use crate::{MemoryEntry, MemoryError, MemoryQuery, ScoredEntry, SemanticQuery};
use async_trait::async_trait;

/// Core abstraction for memory storage backends.
///
/// Implementations must be `Send + Sync + 'static` so they can be shared
/// across async tasks via `Arc`.
///
/// # Provided backends
///
/// | Backend | Feature | Description |
/// |---------|---------|-------------|
/// | [`InMemoryStore`](crate::backend::InMemoryStore) | default | HashMap-based, TTL support |
/// | Redis | `redis` | Production key-value store |
/// | Vector DB | `vector` | Semantic similarity search |
#[async_trait]
pub trait MemoryStore: Send + Sync + 'static {
    /// Store or update a memory entry.
    async fn store(&self, entry: MemoryEntry) -> Result<(), MemoryError>;

    /// Retrieve an entry by key. Returns `None` if not found or expired.
    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>, MemoryError>;

    /// List entries matching a query.
    async fn list(&self, query: &MemoryQuery) -> Result<Vec<MemoryEntry>, MemoryError>;

    /// Delete an entry by key. Returns `true` if it existed.
    async fn delete(&self, key: &str) -> Result<bool, MemoryError>;

    /// Count total entries (optionally filtered by namespace).
    async fn count(&self, namespace: Option<&str>) -> Result<usize, MemoryError>;

    /// Remove all entries (optionally within a namespace).
    async fn clear(&self, namespace: Option<&str>) -> Result<(), MemoryError>;

    /// Clone this store into a boxed trait object (for embedding in other structs).
    fn clone_store(&self) -> Box<dyn MemoryStore>;
}

/// Extended trait for stores that support vector similarity search.
#[async_trait]
pub trait SemanticMemoryStore: MemoryStore {
    /// Search for entries semantically similar to the query embedding.
    async fn search(&self, query: &SemanticQuery) -> Result<Vec<ScoredEntry>, MemoryError>;
}
