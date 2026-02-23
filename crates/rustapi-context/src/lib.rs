//! # rustapi-context
//!
//! AI-Native request context, execution tracing, cost tracking, and event bus
//! for the RustAPI AI Runtime.
//!
//! This crate provides the foundational types that flow through every AI request:
//!
//! - [`RequestContext`] — immutable per-request context carrying auth, metadata,
//!   cost tracking, and observability data
//! - [`TraceTree`] / [`TraceNode`] — hierarchical execution trace recording
//! - [`CostTracker`] — atomic token/cost accounting with budget enforcement
//! - [`EventBus`] — broadcast-based event system for the execution pipeline
//!
//! ## Architecture
//!
//! ```text
//! HTTP Request
//!     │
//!     ▼
//! RequestContext (immutable snapshot)
//!     ├── TraceTree   (append-only execution log)
//!     ├── CostTracker (atomic counters)
//!     ├── AuthContext  (identity / claims)
//!     ├── Metadata     (extensible k-v)
//!     └── EventBus    (broadcast channel)
//! ```

mod context;
mod cost;
mod error;
mod event;
mod trace;

pub use context::*;
pub use cost::*;
pub use error::*;
pub use event::*;
pub use trace::*;
