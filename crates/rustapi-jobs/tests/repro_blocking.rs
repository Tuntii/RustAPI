use async_trait::async_trait;
use rustapi_jobs::{EnqueueOptions, InMemoryBackend, Job, JobContext, JobQueue, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimpleJobData {
    id: i32,
}

#[derive(Clone)]
struct SimpleJob {
    processed_ids: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl Job for SimpleJob {
    const NAME: &'static str = "simple_job";
    type Data = SimpleJobData;

    async fn execute(&self, _ctx: JobContext, data: Self::Data) -> Result<()> {
        self.processed_ids.lock().unwrap().push(data.id);
        Ok(())
    }
}

#[tokio::test]
async fn test_head_of_line_blocking() {
    let backend = InMemoryBackend::new();
    let queue = JobQueue::new(backend);

    let processed_ids = Arc::new(Mutex::new(Vec::new()));
    let job = SimpleJob {
        processed_ids: processed_ids.clone(),
    };

    queue.register_job(job).await;

    // 1. Enqueue a job scheduled far in the future (Job 1)
    let opts_future = EnqueueOptions::new().delay(Duration::from_secs(3600));
    queue
        .enqueue_opts::<SimpleJob>(SimpleJobData { id: 1 }, opts_future)
        .await
        .unwrap();

    // 2. Enqueue a job scheduled now (Job 2)
    queue
        .enqueue::<SimpleJob>(SimpleJobData { id: 2 })
        .await
        .unwrap();

    // 3. Attempt to process one job.
    // Job 2 should be picked up because Job 1 is not ready.
    let result = queue.process_one().await.unwrap();

    // Verify
    if result {
        // If it processed something, it MUST be Job 2
        let ids = processed_ids.lock().unwrap().clone();
        assert_eq!(ids.len(), 1);
        assert_eq!(
            ids[0], 2,
            "Should have processed Job 2, but processed {:?}",
            ids
        );
    } else {
        // If it returned false, it means it was blocked by Job 1
        panic!("Head-of-line blocking detected! Failed to process Job 2 because Job 1 is blocking the queue.");
    }
}
