use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub mod memory;

#[cfg(feature = "redis")]
pub mod redis;

#[cfg(feature = "postgres")]
pub mod postgres;

/// A raw job request to be stored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRequest {
    pub id: String,
    pub name: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
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
