# rustapi-jobs

**Lens**: "The Workhorse"  
**Philosophy**: "Fire and forget, with reliability guarantees."

Background job processing for RustAPI. Long-running tasks shouldn't block HTTP requests.

## Quick Start

```rust
// Define a job
#[derive(Serialize, Deserialize)]
struct EmailJob { to: String }

// Enqueue it
queue.push(EmailJob { to: "alice@example.com" }).await;
```

## Backends

| Backend | Use Case |
|---------|----------|
| **Memory** | Development and testing |
| **Redis** | High throughput persistence |
| **Postgres** | Transactional reliability (ACID) |

## Reliability Features

- **Exponential Backoff**: Automatic retries for failing jobs
- **Dead Letter Queue**: Poison jobs are isolated for manual inspection
- **At-Least-Once Delivery**: Jobs are not lost if a worker crashes
- **Scheduling**: Cron-like recurring tasks

## Full Example

```rust
use rustapi_jobs::{Job, JobContext};

#[derive(Serialize, Deserialize)]
struct SendEmail {
    to: String,
    content: String,
}

#[async_trait]
impl Job for SendEmail {
    const NAME: &'static str = "send_email";

    async fn run(&self, _ctx: JobContext) -> Result<()> {
        // Send the email...
        Ok(())
    }
}

// Enqueue
queue.push(SendEmail { ... }).await?;
```
