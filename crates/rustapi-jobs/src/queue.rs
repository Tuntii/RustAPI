use crate::backend::{JobBackend, JobRequest};
use crate::error::{JobError, Result};
use crate::job::{Job, JobContext, JobHandler};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Main job queue manager
#[derive(Clone)]
pub struct JobQueue {
    backend: Arc<dyn JobBackend>,
    handlers: Arc<RwLock<HashMap<String, Box<dyn JobHandler>>>>,
}

impl JobQueue {
    /// Create a new job queue with a backend
    pub fn new<B: JobBackend + 'static>(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a job handler
    pub async fn register_job<J: Job + Clone>(&self, job: J) {
        let mut handlers = self.handlers.write().await;
        handlers.insert(J::NAME.to_string(), Box::new(job));
    }

    /// Enqueue a job
    pub async fn enqueue<J: Job>(&self, data: J::Data) -> Result<String> {
        self.enqueue_opts::<J>(data, EnqueueOptions::default())
            .await
    }

    /// Enqueue a job with options
    pub async fn enqueue_opts<J: Job>(
        &self,
        data: J::Data,
        opts: EnqueueOptions,
    ) -> Result<String> {
        let payload = serde_json::to_value(data)?;
        let id = Uuid::new_v4().to_string();

        let request = JobRequest {
            id: id.clone(),
            name: J::NAME.to_string(),
            payload,
            created_at: chrono::Utc::now(),
            attempts: 0,
            max_attempts: opts.max_attempts,
            last_error: None,
            run_at: opts.run_at,
        };

        self.backend.push(request).await?;
        Ok(id)
    }

    /// Process a single job (for testing or manual control)
    pub async fn process_one(&self) -> Result<bool> {
        if let Some(req) = self.backend.pop().await? {
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(&req.name) {
                let ctx = JobContext {
                    job_id: req.id.clone(),
                    attempt: req.attempts + 1,
                    created_at: req.created_at,
                };

                match handler.handle(ctx, req.payload.clone()).await {
                    Ok(_) => {
                        self.backend.complete(&req.id).await?;
                        Ok(true)
                    }
                    Err(e) => {
                        let mut new_req = req.clone();
                        new_req.attempts += 1;
                        new_req.last_error = Some(e.to_string());

                        if new_req.attempts < new_req.max_attempts {
                            // Exponential backoff: 2^attempts seconds (e.g. 2, 4, 8, 16...)
                            // Limit max backoff to some reasonable value (e.g. 24 hours)?
                            // For now basic exponential.
                            let backoff_secs = 2u64.saturating_pow(new_req.attempts).min(86400);
                            let retry_delay = chrono::Duration::seconds(backoff_secs as i64);
                            new_req.run_at = Some(chrono::Utc::now() + retry_delay);

                            // Re-push the job for retry
                            self.backend.push(new_req).await?;
                        } else {
                            // Job failed permanently
                            self.backend.fail(&req.id, &e.to_string()).await?;

                            // TODO: If we implemented a real DLQ, we would push it there now.
                            // Currently fail() is where backend would handle that.
                        }
                        Ok(true)
                    }
                }
            } else {
                // Handler not found
                // For now, treat as permanent failure
                self.backend
                    .fail(&req.id, &format!("No handler for job: {}", req.name))
                    .await?;
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    /// Start a worker loop
    pub async fn start_worker(&self) -> Result<()> {
        loop {
            match self.process_one().await {
                Ok(processed) => {
                    if !processed {
                        // Empty queue, sleep a bit
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
                Err(e) => {
                    tracing::error!("Worker error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

/// Options for enqueueing a job
#[derive(Debug, Clone, Default)]
pub struct EnqueueOptions {
    pub max_attempts: u32,
    pub run_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl EnqueueOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n;
        self
    }

    pub fn delay(mut self, duration: std::time::Duration) -> Self {
        self.run_at = Some(chrono::Utc::now() + chrono::Duration::from_std(duration).unwrap());
        self
    }
}
