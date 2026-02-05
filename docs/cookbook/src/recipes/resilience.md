# Resilience Patterns

Building robust applications requires handling failures gracefully. RustAPI provides a suite of middleware to help your service survive partial outages, latency spikes, and transient errors.

These patterns are essential for the "Enterprise Platform" learning path and microservices architectures.

## Prerequisites

Add the resilience features to your `Cargo.toml`. For example:

```toml
[dependencies]
rustapi-rs = { version = "0.1.275", features = ["full"] }
# OR cherry-pick features
# rustapi-extras = { version = "0.1.275", features = ["circuit-breaker", "retry", "timeout"] }
```

## Circuit Breaker

The Circuit Breaker pattern prevents your application from repeatedly trying to execute an operation that's likely to fail. It gives the failing service time to recover.

### How it works
1.  **Closed**: Requests flow normally.
2.  **Open**: After `failure_threshold` is reached, requests fail immediately with `503 Service Unavailable`.
3.  **Half-Open**: After `timeout` passes, a limited number of test requests are allowed. If they succeed, the circuit closes.

### Usage

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::circuit_breaker::CircuitBreakerLayer;
use std::time::Duration;

fn main() {
    let app = RustApi::new()
        .layer(
            CircuitBreakerLayer::new()
                .failure_threshold(5)                // Open after 5 failures
                .timeout(Duration::from_secs(30))    // Wait 30s before retrying
                .success_threshold(2)                // Require 2 successes to close
        )
        .route("/", get(handler));

    // ... run app
}
```

## Retry with Backoff

Transient failures (network blips, temporary timeouts) can often be resolved by simply retrying the request. The `RetryLayer` handles this automatically with configurable backoff strategies.

### Strategies
-   **Exponential**: `base * 2^attempt` (Recommended for most cases)
-   **Linear**: `base * attempt`
-   **Fixed**: Constant delay

### Usage

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::retry::{RetryLayer, RetryStrategy};
use std::time::Duration;

fn main() {
    let app = RustApi::new()
        .layer(
            RetryLayer::new()
                .max_attempts(3)
                .initial_backoff(Duration::from_millis(100))
                .max_backoff(Duration::from_secs(5))
                .strategy(RetryStrategy::Exponential)
                .retryable_statuses(vec![500, 502, 503, 504, 429])
        )
        .route("/", get(handler));

    // ... run app
}
```

> **Warning**: Be careful when combining Retries with non-idempotent operations (like `POST` requests that charge a credit card). The middleware safely handles cloning requests, but your business logic must support it.

## Timeouts

Never let a request hang indefinitely. The `TimeoutLayer` enforces a hard limit on request duration, returning `408 Request Timeout` if exceeded.

### Usage

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::timeout::TimeoutLayer;
use std::time::Duration;

fn main() {
    let app = RustApi::new()
        // Fail if handler takes longer than 5 seconds
        .layer(TimeoutLayer::from_secs(5))
        .route("/", get(slow_handler));

    // ... run app
}
```

## Combining Layers (The Resilience Stack)

Order matters! Timeout should be the "outermost" constraint, followed by Circuit Breaker, then Retry.

In RustAPI (Tower) middleware, layers wrap around each other. The order you call `.layer()` wraps the *previous* service.

**Recommended Order:**
1.  **Retry** (Inner): Retries specific failures from the handler.
2.  **Circuit Breaker** (Middle): Stops retrying if the system is overloaded.
3.  **Timeout** (Outer): Enforces global time limit including all retries.

```rust
let app = RustApi::new()
    // 1. Retry (handles transient errors)
    .layer(RetryLayer::new())
    // 2. Circuit Breaker (protects upstream)
    .layer(CircuitBreakerLayer::new())
    // 3. Timeout (applies to the whole operation)
    .layer(TimeoutLayer::from_secs(10))
    .route("/", get(handler));
```
