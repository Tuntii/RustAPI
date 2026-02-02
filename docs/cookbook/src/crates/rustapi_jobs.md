# rustapi-jobs: The Workhorse

**Lens**: "The Workhorse"
**Philosophy**: "Fire and forget, with reliability guarantees."

## Background Processing

Long-running tasks shouldn't block HTTP requests. `rustapi-jobs` provides a robust queue system that can run in-memory or be backed by Redis/Postgres.

## Usage Example

Here is how to set up a simple background job queue using the in-memory backend.

### 1. Define the Job

Jobs are simple structs that implement `Serialize` and `Deserialize`.

```rust
use serde::{Deserialize, Serialize};
use rustapi_jobs::{Job, JobContext, Result};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EmailJob {
    to: String,
    subject: String,
    body: String,
}

// Implement the Job trait to define how to process it
#[async_trait::async_trait]
impl Job for EmailJob {
    const NAME: &'static str = "email_job";
    type Data = EmailJob;

    async fn execute(_ctx: JobContext, data: Self::Data) -> Result<()> {
        println!("Sending email to {} with subject: {}", data.to, data.subject);
        // Simulate work
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(())
    }
}
```

### 2. Configure the Queue

In your `main` function, initialize the queue and start the worker.

```rust
use rustapi_jobs::{JobQueue, InMemoryBackend, EnqueueOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create the backend
    let backend = InMemoryBackend::new();

    // 2. Create the queue
    let queue = JobQueue::new(backend);

    // 3. Register the job type
    queue.register_job::<EmailJob>();

    // 4. Start the worker in the background
    let worker_queue = queue.clone();
    tokio::spawn(async move {
        if let Err(err) = worker_queue.start_worker().await {
            eprintln!("Job worker exited with error: {err}");
        }
    });

    // 5. Enqueue a job
    queue.enqueue(EmailJob {
        to: "user@example.com".into(),
        subject: "Welcome!".into(),
        body: "Thanks for joining.".into(),
    }).await?;

    Ok(())
}
```

## Backends

- **Memory**: Great for development and testing. Zero infrastructure required.
- **Redis**: High throughput persistence. Recommended for production.
- **Postgres**: Transactional reliability (ACID). Best if you cannot lose jobs.

## Reliability Features

The worker system includes built-in reliability features:

- **Exponential Backoff**: Automatically retries failing jobs with increasing delays.
- **Dead Letter Queue (DLQ)**: "Poison" jobs that fail repeatedly are isolated for manual inspection.
- **Concurrency Control**: Limit the number of concurrent workers to prevent overloading your system.
