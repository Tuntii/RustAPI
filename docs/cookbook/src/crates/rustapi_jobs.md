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

    async fn run(&self, _ctx: JobContext) -> Result<()> {
        println!("Sending email to {} with subject: {}", self.to, self.subject);
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
        worker_queue.start_workers().await;
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

### Redis Backend

Enable the `redis` feature in `Cargo.toml`:

```toml
[dependencies]
rustapi-jobs = { version = "0.1.300", features = ["redis"] }
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
rustapi-jobs = { version = "0.1.300", features = ["postgres"] }
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
