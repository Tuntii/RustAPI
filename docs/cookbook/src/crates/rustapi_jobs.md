# rustapi-jobs: The Workhorse

**Lens**: "The Workhorse"
**Philosophy**: "Fire and forget, with reliability guarantees."

## Background Processing

Long-running tasks shouldn't block HTTP requests. `rustapi-jobs` provides a robust queue system.

```rust
// Define a job
#[derive(Serialize, Deserialize)]
struct EmailJob { to: String }

// Enqueue it
queue.push(EmailJob { to: "alice@example.com" }).await;
```

## Backends

- **Memory**: Great for development and testing.
- **Redis**: High throughput persistence.
- **Postgres**: Transactional reliability (acid).

## Reliability

The worker system features:
- **Exponential Backoff**: Automatic retries for failing jobs.
- **Dead Letter Queue**: Poison jobs are isolated for manual inspection.
