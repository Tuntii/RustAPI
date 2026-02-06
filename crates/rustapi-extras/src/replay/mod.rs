//! Replay middleware for time-travel debugging.
//!
//! This module provides the runtime integration for the replay system:
//!
//! - [`ReplayLayer`] - Middleware that records request/response pairs
//! - [`InMemoryReplayStore`] - In-memory bounded ring buffer store
//! - [`FsReplayStore`] - Filesystem-backed store (JSON Lines)
//! - [`ReplayClient`] - HTTP client for replaying recorded requests
//! - [`RetentionJob`] - Background TTL cleanup task
//! - [`ReplayAdminAuth`] - Bearer token authentication for admin endpoints
//!
//! # Quick Start
//!
//! ```ignore
//! use rustapi_extras::replay::ReplayLayer;
//! use rustapi_core::replay::ReplayConfig;
//!
//! let layer = ReplayLayer::new(
//!     ReplayConfig::new()
//!         .enabled(true)
//!         .admin_token("my-secret-token")
//! );
//!
//! RustApi::new()
//!     .layer(layer)
//!     .route("/api/users", get(handler))
//!     .run("127.0.0.1:8080")
//!     .await?;
//! ```

mod auth;
mod client;
mod fs_store;
mod layer;
mod memory_store;
mod retention;
mod routes;

pub use auth::ReplayAdminAuth;
pub use client::{ReplayClient, ReplayClientError};
pub use fs_store::{FsReplayStore, FsReplayStoreConfig};
pub use layer::ReplayLayer;
pub use memory_store::InMemoryReplayStore;
pub use retention::RetentionJob;
