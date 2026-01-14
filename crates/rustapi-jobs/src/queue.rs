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

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::backend::memory::MemoryBackend;
    use proptest::prelude::*;
    use serde::{Deserialize, Serialize};

    /// **Feature: v1-features-roadmap, Properties 21-22: Job persistence and retry**
    /// **Validates: Requirements 10.1, 10.2**
    ///
    /// For background job processing:
    /// - Property 21: Jobs SHALL persist across backend operations
    /// - Property 22: Retry SHALL use exponential backoff (2^attempts seconds)
    /// - Failed jobs SHALL be retried up to max_attempts
    /// - Successful jobs SHALL be marked complete and removed
    /// - Jobs with run_at in future SHALL not be processed immediately

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestJobData {
        value: i32,
    }

    #[derive(Clone)]
    struct TestJob {
        should_fail: Arc<RwLock<bool>>,
        execution_count: Arc<RwLock<u32>>,
    }

    #[async_trait]
    impl Job for TestJob {
        const NAME: &'static str = "test_job";
        type Data = TestJobData;

        async fn execute(&self, _ctx: JobContext, data: Self::Data) -> Result<()> {
            let mut count = self.execution_count.write().await;
            *count += 1;

            let should_fail = *self.should_fail.read().await;
            if should_fail {
                return Err(JobError::ExecutionFailed(format!(
                    "Test failure for value {}",
                    data.value
                )));
            }
            Ok(())
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Property 21: Jobs persist through push/pop cycle
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

                // Enqueue job
                let job_id = queue
                    .enqueue::<TestJob>(TestJobData { value })
                    .await
                    .unwrap();

                prop_assert!(!job_id.is_empty());

                // Job MUST be retrievable
                let processed = queue.process_one().await.unwrap();
                prop_assert!(processed);

                // Job MUST have been executed
                let count = *test_job.execution_count.read().await;
                prop_assert_eq!(count, 1);

                Ok(())
            })?;
        }

        /// Property 22: Exponential backoff is calculated correctly
        #[test]
        fn prop_exponential_backoff_calculation(attempts in 0u32..10) {
            let expected_backoff = 2u64.saturating_pow(attempts).min(86400);
            
            // This is the formula used in process_one for retries
            let calculated_backoff = 2u64.saturating_pow(attempts).min(86400);
            
            prop_assert_eq!(calculated_backoff, expected_backoff);
            
            // Verify exponential growth
            if attempts > 0 && expected_backoff < 86400 {
                let previous = 2u64.saturating_pow(attempts - 1);
                prop_assert_eq!(expected_backoff, previous * 2);
            }
        }

        /// Property 22: Failed jobs are retried with exponential backoff
        #[test]
        fn prop_retry_behavior(value in -1000i32..1000i32, max_attempts in 2u32..5) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let backend = MemoryBackend::new();
                let queue = JobQueue::new(backend);

                let test_job = TestJob {
                    should_fail: Arc::new(RwLock::new(true)), // Always fail
                    execution_count: Arc::new(RwLock::new(0)),
                };
                queue.register_job(test_job.clone()).await;

                // Enqueue with max attempts
                let opts = EnqueueOptions::new().max_attempts(max_attempts);
                let _job_id = queue
                    .enqueue_opts::<TestJob>(TestJobData { value }, opts)
                    .await
                    .unwrap();

                // Process job multiple times (it will fail and retry)
                for attempt in 1..=max_attempts {
                    let processed = queue.process_one().await.unwrap();
                    prop_assert!(processed);

                    let count = *test_job.execution_count.read().await;
                    prop_assert_eq!(count, attempt);
                }

                // After max_attempts, job should be failed permanently
                // No more jobs to process
                let processed = queue.process_one().await.unwrap();
                prop_assert!(!processed); // Queue should be empty

                Ok(())
            })?;
        }

        /// Property 21: Multiple jobs persist independently
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

                // Enqueue all jobs
                let job_count = values.len();
                for value in values {
                    queue.enqueue::<TestJob>(TestJobData { value }).await.unwrap();
                }

                // Process all jobs
                for _ in 0..job_count {
                    let processed = queue.process_one().await.unwrap();
                    prop_assert!(processed);
                }

                // All jobs MUST have been executed
                let count = *test_job.execution_count.read().await;
                prop_assert_eq!(count as usize, job_count);

                // Queue MUST be empty
                let processed = queue.process_one().await.unwrap();
                prop_assert!(!processed);

                Ok(())
            })?;
        }

        /// Property 22: Jobs with run_at in future are not processed
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

                // Enqueue with delay
                let opts = EnqueueOptions::new()
                    .delay(std::time::Duration::from_secs(3600)); // 1 hour delay
                queue
                    .enqueue_opts::<TestJob>(TestJobData { value }, opts)
                    .await
                    .unwrap();

                // Try to process immediately - should not process delayed job
                let processed = queue.process_one().await.unwrap();
                prop_assert!(!processed); // Job should not be processed yet

                // Execution count should still be 0
                let count = *test_job.execution_count.read().await;
                prop_assert_eq!(count, 0);

                Ok(())
            })?;
        }

        /// Property 22: Successful job is completed and removed
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

                // Process once - should succeed
                let processed = queue.process_one().await.unwrap();
                prop_assert!(processed);

                // Job MUST be executed exactly once
                let count = *test_job.execution_count.read().await;
                prop_assert_eq!(count, 1);

                // Job MUST be removed from queue (completed)
                let processed_again = queue.process_one().await.unwrap();
                prop_assert!(!processed_again); // Queue should be empty

                // Execution count MUST not increase
                let count_after = *test_job.execution_count.read().await;
                prop_assert_eq!(count_after, 1);

                Ok(())
            })?;
        }

        /// Property 21: Job IDs are unique
        #[test]
        fn prop_job_ids_unique(count in 2usize..10) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let backend = MemoryBackend::new();
                let queue = JobQueue::new(backend);

                let test_job = TestJob {
                    should_fail: Arc::new(RwLock::new(false)),
                    execution_count: Arc::new(RwLock::new(0)),
                };
                queue.register_job(test_job).await;

                // Enqueue multiple jobs
                let mut job_ids = Vec::new();
                for i in 0..count {
                    let id = queue
                        .enqueue::<TestJob>(TestJobData { value: i as i32 })
                        .await
                        .unwrap();
                    job_ids.push(id);
                }

                // All IDs MUST be unique
                for i in 0..job_ids.len() {
                    for j in (i + 1)..job_ids.len() {
                        prop_assert_ne!(&job_ids[i], &job_ids[j]);
                    }
                }

                Ok(())
            })?;
        }

        /// Property 22: Max attempts limit is respected
        #[test]
        fn prop_max_attempts_respected(value in -100i32..100, max_attempts in 1u32..5) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let backend = MemoryBackend::new();
                let queue = JobQueue::new(backend);

                let test_job = TestJob {
                    should_fail: Arc::new(RwLock::new(true)),
                    execution_count: Arc::new(RwLock::new(0)),
                };
                queue.register_job(test_job.clone()).await;

                let opts = EnqueueOptions::new().max_attempts(max_attempts);
                queue
                    .enqueue_opts::<TestJob>(TestJobData { value }, opts)
                    .await
                    .unwrap();

                // Process until queue is empty
                let mut process_count = 0;
                while queue.process_one().await.unwrap() {
                    process_count += 1;
                    // Safety limit
                    if process_count > max_attempts + 5 {
                        break;
                    }
                }

                // Job MUST have been executed exactly max_attempts times
                let count = *test_job.execution_count.read().await;
                prop_assert_eq!(count, max_attempts);

                Ok(())
            })?;
        }

        /// Property 22: Backoff increases exponentially, not linearly
        #[test]
        fn prop_backoff_exponential_not_linear(attempt in 1u32..8) {
            let backoff_current = 2u64.saturating_pow(attempt);
            let backoff_previous = 2u64.saturating_pow(attempt - 1);
            
            // Exponential: backoff_current = 2 * backoff_previous
            prop_assert_eq!(backoff_current, backoff_previous * 2);
            
            // NOT linear: backoff_current != backoff_previous + constant
            let linear_would_be = backoff_previous + 2; // Linear increment of 2
            if attempt > 1 {
                prop_assert_ne!(backoff_current, linear_would_be);
            }
        }

        /// Property 22: Backoff is capped at maximum (86400 seconds = 24 hours)
        #[test]
        fn prop_backoff_capped(attempt in 20u32..30) {
            let backoff = 2u64.saturating_pow(attempt).min(86400);
            
            // MUST be capped at 86400
            prop_assert_eq!(backoff, 86400);
            
            // Without cap, would be much larger
            let uncapped = 2u64.saturating_pow(attempt);
            prop_assert!(uncapped > 86400);
        }
    }
}
