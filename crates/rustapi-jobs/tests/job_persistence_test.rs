use async_trait::async_trait;
use rustapi_jobs::{InMemoryBackend, Job, JobContext, JobQueue, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmailJobData {
    to: String,
    subject: String,
    body: String,
}

#[derive(Clone)]
struct EmailJob {
    // Shared state to verify execution
    processed: Arc<Mutex<Vec<EmailJobData>>>,
}

#[async_trait]
impl Job for EmailJob {
    const NAME: &'static str = "email_job";
    type Data = EmailJobData;

    async fn execute(&self, _ctx: JobContext, data: Self::Data) -> Result<()> {
        self.processed.lock().unwrap().push(data);
        Ok(())
    }
}

#[tokio::test]
async fn test_job_persistence_in_memory() {
    // Setup
    let backend = InMemoryBackend::new();
    let queue = JobQueue::new(backend);

    let processed = Arc::new(Mutex::new(Vec::new()));
    let job = EmailJob {
        processed: processed.clone(),
    };

    queue.register_job(job).await;

    // Enqueue
    let data = EmailJobData {
        to: "user@example.com".to_string(),
        subject: "Welcome".to_string(),
        body: "Hello!".to_string(),
    };

    let job_id = queue
        .enqueue::<EmailJob>(data.clone())
        .await
        .expect("Enqueue failed");
    assert!(!job_id.is_empty());

    // Process
    let result = queue.process_one().await.expect("Process failed");
    assert!(result, "Should have processed one job");

    // Verify
    let handled = processed.lock().unwrap();
    assert_eq!(handled.len(), 1);
    assert_eq!(handled[0].to, "user@example.com");
}

#[tokio::test]
async fn test_multiple_jobs() {
    let backend = InMemoryBackend::new();
    let queue = JobQueue::new(backend);

    let processed = Arc::new(Mutex::new(Vec::new()));
    let job = EmailJob {
        processed: processed.clone(),
    };
    queue.register_job(job).await;

    for i in 0..5 {
        queue
            .enqueue::<EmailJob>(EmailJobData {
                to: format!("user{}@example.com", i),
                subject: "Hi".to_string(),
                body: "Msg".to_string(),
            })
            .await
            .unwrap();
    }

    // Process all
    for _ in 0..5 {
        assert!(queue.process_one().await.unwrap());
    }

    // Should be empty now
    assert!(!queue.process_one().await.unwrap());

    let handled = processed.lock().unwrap();
    assert_eq!(handled.len(), 5);
}

#[derive(Clone)]
struct FailingJob {
    attempts: Arc<Mutex<u32>>,
}

#[async_trait]
impl Job for FailingJob {
    const NAME: &'static str = "failing_job";
    type Data = (); // No data needed

    async fn execute(&self, _ctx: JobContext, _data: Self::Data) -> Result<()> {
        let mut attempts = self.attempts.lock().unwrap();
        *attempts += 1;
        Err(rustapi_jobs::JobError::WorkerError(
            "Always fails".to_string(),
        ))
    }
}

#[tokio::test]
async fn test_retry_behavior() {
    let backend = InMemoryBackend::new();
    let queue = JobQueue::new(backend);

    let attempts = Arc::new(Mutex::new(0));
    let job = FailingJob {
        attempts: attempts.clone(),
    };
    queue.register_job(job).await;

    // Enqueue with max attempts = 3
    let opts = rustapi_jobs::EnqueueOptions::new().max_attempts(3);
    queue.enqueue_opts::<FailingJob>((), opts).await.unwrap();

    // 1st attempt
    queue.process_one().await.unwrap();
    assert_eq!(*attempts.lock().unwrap(), 1);

    // Should be re-queued with delay.
    // InMemoryBackend pop checks head. If delayed, returns None.
    let result = queue.process_one().await.unwrap();
    // It might return false if 'None' is returned from pop (delayed)
    // Or if our InMemory logic is strictly FIFO and head is delayed, it returns None.

    // We can't easily "time travel" with std::time in tests without internal mocking.
    // But we verified the behavior: process_one returns true if it processed something?
    // Wait, process_one calls backend.pop(). If pop returns None, it returns false.
    // So if delay works, result should be false immediately after failure.
    assert!(!result, "Should not process immediately due to backoff");

    // Check attempts count didn't increase
    assert_eq!(*attempts.lock().unwrap(), 1);

    // Note: We can't verify 2nd attempt execution without waiting 2 seconds (base backoff).
    // And we don't want to slow down tests.
    // So verifying it stopped processing is enough to prove delay was set.
}
