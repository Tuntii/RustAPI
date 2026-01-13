//! Background job processing for RustAPI
//!
//! r# "RustAPI Jobs"
//!
//! This crate provides a flexible background job processing system.

pub mod backend;
pub mod error;
pub mod job;
pub mod queue;

pub use backend::memory::InMemoryBackend;
pub use backend::{JobBackend, JobRequest};
pub use error::{JobError, Result};
pub use job::{Job, JobContext};
pub use queue::{EnqueueOptions, JobQueue};
