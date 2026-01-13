use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

/// Backend storage for jobs
#[async_trait]
pub trait JobBackend: Send + Sync {
    /// Push a new job to the queue
    async fn push(&self, job: JobRequest) -> Result<()>;

    /// Pop the next available job
    /// Should return None if no job is available or ready
    async fn pop(&self) -> Result<Option<JobRequest>>;

    /// Mark a job as completed successfully
    async fn complete(&self, job_id: &str) -> Result<()>;

    /// Mark a job as failed
    /// The manager will decide whether to retry (re-push) or move to DLQ
    async fn fail(&self, job_id: &str, error: &str) -> Result<()>;
}
