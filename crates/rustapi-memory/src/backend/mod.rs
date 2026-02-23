//! Backend implementations for [`MemoryStore`](crate::MemoryStore).

mod memory;

#[cfg(feature = "redis")]
mod redis;

pub use memory::InMemoryStore;

#[cfg(feature = "redis")]
pub use self::redis::RedisStore;
