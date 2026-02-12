# Background Jobs

RustAPI provides a robust background job processing system through the `rustapi-jobs` crate. This allows you to offload time-consuming tasks (like sending emails, processing images, or generating reports) from the main request/response cycle, keeping your API fast and responsive.

## Setup

First, add `rustapi-jobs` to your `Cargo.toml`. Since `rustapi-jobs` is not re-exported by the main crate by default, you must include it explicitly.

```toml
[dependencies]
rustapi-rs = "0.1"
rustapi-jobs = "0.1"
serde = { version = "1.0", features = ["derive"] }
async-trait = "0.1"
tokio = { version = "1.0", features = ["full"] }
```

## Defining a Job

A job consists of a data structure (the payload) and an implementation of the `Job` trait.

```rust,no_run
use rustapi_jobs::{Job, JobContext, Result};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use std::fmt::Debug;

// 1. Define the job payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WelcomeEmailData {
    pub user_id: String,
    pub email: String,
}

// 2. Define the job handler struct
#[derive(Clone)]
pub struct WelcomeEmailJob;

// 3. Implement the Job trait
#[async_trait]
impl Job for WelcomeEmailJob {
    // Unique name for the job type
    const NAME: &'static str = "send_welcome_email";

    // The payload type
    type Data = WelcomeEmailData;

    async fn execute(&self, ctx: JobContext, data: Self::Data) -> Result<()> {
        println!("Processing job {} (attempt {})", ctx.job_id, ctx.attempt);
        println!("Sending welcome email to {} ({})", data.email, data.user_id);

        // Simulate work
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok(())
    }
}
```

## Registering and Running the Queue

In your main application setup, you need to:
1. Initialize the backend (Memory, Redis, or Postgres).
2. Create the `JobQueue`.
3. Register your job handlers.
4. Start the worker loop in a background task.
5. Add the `JobQueue` to your application state so handlers can use it.

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_jobs::{JobQueue, InMemoryBackend};
// use crate::jobs::{WelcomeEmailJob, WelcomeEmailData}; // Import your job

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // 1. Initialize backend
    // For production, use Redis or Postgres backend
    let backend = InMemoryBackend::new();

    // 2. Create queue
    let queue = JobQueue::new(backend);

    // 3. Register jobs
    // You must register an instance of the job handler
    queue.register_job(WelcomeEmailJob).await;

    // 4. Start worker in background
    let queue_for_worker = queue.clone();
    tokio::spawn(async move {
        if let Err(e) = queue_for_worker.start_worker().await {
            eprintln!("Worker failed: {}", e);
        }
    });

    // 5. Build application
    RustApi::auto()
        .with_state(queue) // Inject queue into state
        .serve("127.0.0.1:3000")
        .await
}
```

## Enqueueing Jobs

You can now inject the `JobQueue` into your request handlers using the `State` extractor and enqueue jobs.

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_jobs::JobQueue;

#[rustapi::post("/register")]
async fn register_user(
    State(queue): State<JobQueue>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // ... logic to create user in DB ...
    let user_id = "user_123".to_string(); // Simulated ID

    // Enqueue the background job
    // The queue will handle serialization and persistence
    queue.enqueue::<WelcomeEmailJob>(WelcomeEmailData {
        user_id,
        email: payload.email,
    }).await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(json!({
        "status": "registered",
        "message": "Welcome email will be sent shortly"
    })))
}

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
}
```

## Resilience and Retries

`rustapi-jobs` handles failures automatically. If your `execute` method returns an `Err`, the job will be:
1. Marked as failed.
2. Scheduled for retry with **exponential backoff**.
3. Retried up to `max_attempts` (default is configurable per enqueue).

To customize retry behavior, use `enqueue_opts`:

```rust,no_run
use rustapi_jobs::EnqueueOptions;

queue.enqueue_opts::<WelcomeEmailJob>(
    data,
    EnqueueOptions::new()
        .max_attempts(5) // Retry up to 5 times
        .delay(std::time::Duration::from_secs(60)) // Initial delay
).await?;
```

## Backends

While `InMemoryBackend` is great for testing and simple apps, production systems should use persistent backends:

- **Redis**: High performance, good for volatile queues. Enable `redis` feature in `rustapi-jobs`.
- **Postgres**: Best for reliability and transactional safety. Enable `postgres` feature.

```toml
# In Cargo.toml
rustapi-jobs = { version = "0.1", features = ["redis"] }
```
