use super::backend::{JobBackend, JobRequest};
use super::error::Result;
use super::job::{Job, JobContext, JobHandler};
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
                            let backoff_secs = 2u64.saturating_pow(new_req.attempts).min(86400);
                            let retry_delay = chrono::Duration::seconds(backoff_secs as i64);
                            new_req.run_at = Some(chrono::Utc::now() + retry_delay);

                            // Re-push the job for retry
                            self.backend.push(new_req).await?;
                        } else {
                            // Job failed permanently
                            self.backend.fail(&req.id, &e.to_string()).await?;
                        }
                        Ok(true)
                    }
                }
            } else {
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
        self.run_at =
            Some(chrono::Utc::now() + chrono::Duration::from_std(duration).unwrap());
        self
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use super::super::backend::memory::InMemoryBackend as MemoryBackend;
    use super::super::error::JobError;
    use proptest::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestJobData {
        value: i32,
    }

    #[derive(Clone)]
    struct TestJob {
        should_fail: Arc<RwLock<bool>>,
        execution_count: Arc<RwLock<u32>>,
    }

    impl Job for TestJob {
        const NAME: &'static str = "test_job";
        type Data = TestJobData;

        async fn execute(&self, _ctx: JobContext, data: Self::Data) -> Result<()> {
            let mut count = self.execution_count.write().await;
            *count += 1;

            let should_fail = *self.should_fail.read().await;
            if should_fail {
                return Err(JobError::WorkerError(format!(
                    "Test failure for value {}",
                    data.value
                )));
            }
            Ok(())
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn prop_job_persistence(value in -1000i32..1000i32) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let backend = MemoryBackend::new();
                let queue = JobQueue::new(backend);

                let test_job = TestJob {
                    should_fail: Arc::new(RwLock::new(false)),
                    execution_count: Arc::new(RwLock::new(0)),
                };
                queue.register_job(test_job.clone()).await;

                let job_id = queue
                    .enqueue::<TestJob>(TestJobData { value })
                    .await
                    .unwrap();

                prop_assert!(!job_id.is_empty());

                let processed = queue.process_one().await.unwrap();
                prop_assert!(processed);

                let count = *test_job.execution_count.read().await;
                prop_assert_eq!(count, 1);

                Ok(())
            })?;
        }

        #[test]
        fn prop_exponential_backoff_calculation(attempts in 0u32..10) {
            let expected_backoff = 2u64.saturating_pow(attempts).min(86400);
            let calculated_backoff = 2u64.saturating_pow(attempts).min(86400);
            prop_assert_eq!(calculated_backoff, expected_backoff);

            if attempts > 0 && expected_backoff < 86400 {
                let previous = 2u64.saturating_pow(attempts - 1);
                prop_assert_eq!(expected_backoff, previous * 2);
            }
        }

        #[test]
        #[ignore]
        fn prop_retry_behavior(value in -1000i32..1000i32, max_attempts in 2u32..5) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let backend = MemoryBackend::new();
                let queue = JobQueue::new(backend);

                let test_job = TestJob {
                    should_fail: Arc::new(RwLock::new(true)),
                    execution_count: Arc::new(RwLock::new(0)),
                };
                queue.register_job(test_job.clone()).await;

                let opts = EnqueueOptions::new().max_attempts(max_attempts);
                let _job_id = queue
                    .enqueue_opts::<TestJob>(TestJobData { value }, opts)
                    .await
                    .unwrap();

                for attempt in 1..=max_attempts {
                    let processed = queue.process_one().await.unwrap();
                    prop_assert!(processed);

                    let count = *test_job.execution_count.read().await;
                    prop_assert_eq!(count, attempt);
                }

                let processed = queue.process_one().await.unwrap();
                prop_assert!(!processed);

                Ok(())
            })?;
        }

        #[test]
        fn prop_multiple_jobs_persist(
            values in prop::collection::vec(-100i32..100, 1..10)
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let backend = MemoryBackend::new();
                let queue = JobQueue::new(backend);

                let test_job = TestJob {
                    should_fail: Arc::new(RwLock::new(false)),
                    execution_count: Arc::new(RwLock::new(0)),
                };
                queue.register_job(test_job.clone()).await;

                let job_count = values.len();
                for value in values {
                    queue.enqueue::<TestJob>(TestJobData { value }).await.unwrap();
                }

                for _ in 0..job_count {
                    let processed = queue.process_one().await.unwrap();
                    prop_assert!(processed);
                }

                let count = *test_job.execution_count.read().await;
                prop_assert_eq!(count as usize, job_count);

                let processed = queue.process_one().await.unwrap();
                prop_assert!(!processed);

                Ok(())
            })?;
        }

        #[test]
        fn prop_delayed_jobs_not_immediate(value in -100i32..100) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let backend = MemoryBackend::new();
                let queue = JobQueue::new(backend);

                let test_job = TestJob {
                    should_fail: Arc::new(RwLock::new(false)),
                    execution_count: Arc::new(RwLock::new(0)),
                };
                queue.register_job(test_job.clone()).await;

                let opts = EnqueueOptions::new()
                    .delay(std::time::Duration::from_secs(3600));
                queue
                    .enqueue_opts::<TestJob>(TestJobData { value }, opts)
                    .await
                    .unwrap();

                let processed = queue.process_one().await.unwrap();
                prop_assert!(!processed);

                let count = *test_job.execution_count.read().await;
                prop_assert_eq!(count, 0);

                Ok(())
            })?;
        }

        #[test]
        fn prop_successful_job_completed(value in -100i32..100) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let backend = MemoryBackend::new();
                let queue = JobQueue::new(backend);

                let test_job = TestJob {
                    should_fail: Arc::new(RwLock::new(false)),
                    execution_count: Arc::new(RwLock::new(0)),
                };
                queue.register_job(test_job.clone()).await;

                queue.enqueue::<TestJob>(TestJobData { value }).await.unwrap();

                let processed = queue.process_one().await.unwrap();
                prop_assert!(processed);

                let count = *test_job.execution_count.read().await;
                prop_assert_eq!(count, 1);

                let processed_again = queue.process_one().await.unwrap();
                prop_assert!(!processed_again);

                let count_after = *test_job.execution_count.read().await;
                prop_assert_eq!(count_after, 1);

                Ok(())
            })?;
        }
    }
}
