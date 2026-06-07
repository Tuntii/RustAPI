use super::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// In-memory job backend implementation.
pub mod memory;

#[cfg(feature = "jobs-redis")]
/// Redis-backed job backend implementation.
pub mod redis;

#[cfg(feature = "jobs-postgres")]
/// PostgreSQL-backed job backend implementation.
pub mod postgres;

/// A raw job request to be stored in the queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRequest {
    /// Unique job identifier.
    pub id: String,
    /// Job type name (matches [`Job::NAME`](super::job::Job::NAME)).
    pub name: String,
    /// Serialized job payload.
    pub payload: serde_json::Value,
    /// When the job was created.
    pub created_at: DateTime<Utc>,
    /// Number of execution attempts so far.
    pub attempts: u32,
    /// Maximum number of execution attempts before permanent failure.
    pub max_attempts: u32,
    /// Error message from the last failed attempt, if any.
    pub last_error: Option<String>,
    /// Earliest time the job should be executed (None = immediately).
    pub run_at: Option<DateTime<Utc>>,
}

/// Backend storage for jobs (dyn-compatible via boxed futures)
pub trait JobBackend: Send + Sync {
    /// Push a new job to the queue
    fn push<'a>(&'a self, job: JobRequest)
        -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// Pop the next available job
    /// Should return None if no job is available or ready
    fn pop<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<Option<JobRequest>>> + Send + 'a>>;

    /// Mark a job as completed successfully
    fn complete<'a>(
        &'a self,
        job_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// Mark a job as failed
    /// The manager will decide whether to retry (re-push) or move to DLQ
    fn fail<'a>(
        &'a self,
        job_id: &'a str,
        error: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}
