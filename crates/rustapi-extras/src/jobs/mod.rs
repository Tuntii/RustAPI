//! Background job processing for RustAPI
//!
//! This module provides a flexible background job processing system
//! with support for in-memory, Redis, and PostgreSQL backends.

/// Backend storage implementations for job queues.
pub mod backend;
/// Error types for job processing.
pub mod error;
/// Job trait and context definitions.
pub mod job;
/// Job queue manager and enqueue options.
pub mod queue;

pub use self::backend::memory::InMemoryBackend;
pub use self::backend::{JobBackend, JobRequest};
pub use self::error::JobError;
pub use self::job::{Job, JobContext};
pub use self::queue::{EnqueueOptions, JobQueue};
