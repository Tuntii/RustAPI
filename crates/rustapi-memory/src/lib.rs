//! # rustapi-memory
//!
//! Pluggable memory layer for the RustAPI AI Runtime.
//!
//! Provides trait-based abstractions for storing, retrieving, and searching
//! memory entries used by AI agents during execution. Backends are
//! pluggable through the [`MemoryStore`] trait.
//!
//! ## Backends
//!
//! | Backend | Feature | Use case |
//! |---------|---------|----------|
//! | [`InMemoryStore`] | default | Development, testing |
//! | Redis | `redis` | Production key-value |
//! | Vector DB | `vector` | Semantic search (pgvector, qdrant) |
//!
//! ## Architecture
//!
//! ```text
//! Agent Engine
//!     │
//!     ▼
//! ConversationMemory ─── session-scoped multi-turn history
//!     │
//!     ▼
//! MemoryStore trait  ─── pluggable backend abstraction
//!     │
//!     ├── InMemoryStore  (DashMap, TTL, dev/test)
//!     ├── RedisStore     (production k-v)
//!     └── VectorStore    (semantic search)
//! ```

mod conversation;
mod entry;
mod error;
mod store;

pub mod backend;

pub use conversation::*;
pub use entry::*;
pub use error::*;
pub use store::*;
