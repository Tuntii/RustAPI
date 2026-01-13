use super::{JobBackend, JobRequest};
use crate::error::{JobError, Result};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// In-memory job backend (not persistent, for testing/dev)
#[derive(Debug, Clone, Default)]
pub struct InMemoryBackend {
    queue: Arc<Mutex<VecDeque<JobRequest>>>,
    // In a real system we'd track processing jobs separately for reliability
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl JobBackend for InMemoryBackend {
    async fn push(&self, job: JobRequest) -> Result<()> {
        let mut q = self
            .queue
            .lock()
            .map_err(|_| JobError::BackendError("Lock poisoned".to_string()))?;
        q.push_back(job);
        Ok(())
    }

    async fn pop(&self) -> Result<Option<JobRequest>> {
        let mut q = self
            .queue
            .lock()
            .map_err(|_| JobError::BackendError("Lock poisoned".to_string()))?;

        // Simple FIFO for now, ignoring run_at logic complexity for basic in-memory
        // In reality we should scan for ready jobs
        if let Some(job) = q.front() {
            if let Some(run_at) = job.run_at {
                if run_at > chrono::Utc::now() {
                    return Ok(None);
                }
            }
        } else {
            return Ok(None);
        }

        Ok(q.pop_front())
    }

    async fn complete(&self, _job_id: &str) -> Result<()> {
        // No-op for simple in-memory queue that removes on pop
        Ok(())
    }

    async fn fail(&self, _job_id: &str, _error: &str) -> Result<()> {
        // In a real implementation we might move to DLQ or re-queue
        Ok(())
    }
}
