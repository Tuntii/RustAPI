use super::{JobBackend, JobRequest};
use crate::error::{JobError, Result};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
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

impl JobBackend for InMemoryBackend {
    fn push<'a>(
        &'a self,
        job: JobRequest,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut q = self
                .queue
                .lock()
                .map_err(|_| JobError::BackendError("Lock poisoned".to_string()))?;
            q.push_back(job);
            Ok(())
        })
    }

    fn pop<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<JobRequest>>> + Send + 'a>> {
        Box::pin(async move {
            let mut q = self
                .queue
                .lock()
                .map_err(|_| JobError::BackendError("Lock poisoned".to_string()))?;

            let now = chrono::Utc::now();
            let mut index_to_remove = None;

            // Scan the queue for the first ready job
            for (i, job) in q.iter().enumerate() {
                if let Some(run_at) = job.run_at {
                    if run_at > now {
                        continue;
                    }
                }
                // Found a ready job (no run_at, or run_at <= now)
                index_to_remove = Some(i);
                break;
            }

            if let Some(i) = index_to_remove {
                Ok(q.remove(i))
            } else {
                Ok(None)
            }
        })
    }

    fn complete<'a>(
        &'a self,
        _job_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        // No-op for simple in-memory queue that removes on pop
        Box::pin(async move { Ok(()) })
    }

    fn fail<'a>(
        &'a self,
        _job_id: &'a str,
        _error: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        // In a real implementation we might move to DLQ or re-queue
        Box::pin(async move { Ok(()) })
    }
}
