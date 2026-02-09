# rustapi-jobs: The Workhorse

**Lens**: "The Workhorse"
**Philosophy**: "Fire and forget, with reliability guarantees."

## Background Processing

Long-running tasks shouldn't block HTTP requests. `rustapi-jobs` provides a robust queue system that can run in-memory or be backed by Redis/Postgres.

## Usage Example

Here is how to set up a simple background job queue using the in-memory backend.

### 1. Define the Job and Data

Jobs are separated into two parts:
1. The **Data** struct (the payload), which must be serializable.
2. The **Job** struct (the handler), which contains the logic.

```rust
use serde::{Deserialize, Serialize};
use rustapi_jobs::{Job, JobContext, Result};
use async_trait::async_trait;

// 1. The payload data
#[derive(Serialize, Deserialize, Debug, Clone)]
struct EmailJobData {
    to: String,
    subject: String,
    body: String,
}

// 2. The handler struct (usually stateless)
#[derive(Clone)]
struct EmailJob;

#[async_trait]
impl Job for EmailJob {
    const NAME: &'static str = "email_job";
    type Data = EmailJobData;

    async fn execute(&self, _ctx: JobContext, data: Self::Data) -> Result<()> {
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
use rustapi_jobs::{JobQueue, InMemoryBackend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create the backend
    let backend = InMemoryBackend::new();

    // 2. Create the queue
    let queue = JobQueue::new(backend);

    // 3. Register the job handler
    queue.register_job(EmailJob).await;

    // 4. Start the worker in the background
    let worker_queue = queue.clone();
    tokio::spawn(async move {
        if let Err(e) = worker_queue.start_worker().await {
            eprintln!("Worker failed: {:?}", e);
        }
    });

    // 5. Enqueue a job (pass the DATA, not the handler)
    queue.enqueue::<EmailJob>(EmailJobData {
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

### Redis Backend

Enable the `redis` feature in `Cargo.toml`:

```toml
[dependencies]
rustapi-jobs = { version = "0.1.335", features = ["redis"] }
```

```rust
use rustapi_jobs::backend::redis::RedisBackend;

let backend = RedisBackend::new("redis://127.0.0.1:6379").await?;
let queue = JobQueue::new(backend);
```

### Postgres Backend

Enable the `postgres` feature in `Cargo.toml`. This uses `sqlx`.

```toml
[dependencies]
rustapi-jobs = { version = "0.1.335", features = ["postgres"] }
```

```rust
use rustapi_jobs::backend::postgres::PostgresBackend;
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new().connect("postgres://user:pass@localhost/db").await?;
let backend = PostgresBackend::new(pool);

// Ensure the jobs table exists
backend.migrate().await?;

let queue = JobQueue::new(backend);
```

## Reliability Features

The worker system includes built-in reliability features:

- **Exponential Backoff**: Automatically retries failing jobs with increasing delays.
- **Dead Letter Queue (DLQ)**: "Poison" jobs that fail repeatedly are isolated for manual inspection.
- **Concurrency Control**: Limit the number of concurrent workers to prevent overloading your system.
