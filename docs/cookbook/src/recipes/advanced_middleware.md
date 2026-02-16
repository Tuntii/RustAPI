# Advanced Middleware: Rate Limiting, Caching, and Deduplication

As your API grows, you'll need to protect it from abuse and optimize performance. RustAPI provides a suite of advanced middleware in `rustapi-extras` to handle these concerns efficiently.

These patterns are essential for the "Enterprise Platform" learning path and high-traffic services.

## Prerequisites

Add the `rustapi-extras` crate with the necessary features to your `Cargo.toml`.

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["full"] }
# OR cherry-pick features
# rustapi-extras = { version = "0.1.335", features = ["rate-limit", "dedup", "cache"] }
```

## Rate Limiting

Rate limiting protects your API from being overwhelmed by too many requests from a single client. It uses a "Token Bucket" or "Fixed Window" algorithm to enforce limits.

### How it works
The `RateLimitLayer` tracks request counts per IP address. When a limit is exceeded, it returns `429 Too Many Requests` with a `Retry-After` header.

### Usage

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::rate_limit::RateLimitLayer;
use std::time::Duration;

fn main() {
    let app = RustApi::new()
        .layer(
            RateLimitLayer::new(100, Duration::from_secs(60)) // 100 requests per minute
        )
        .route("/", get(handler));

    // ... run app
}
```

The middleware automatically adds standard headers to responses:
- `X-RateLimit-Limit`: The maximum number of requests allowed.
- `X-RateLimit-Remaining`: The number of requests remaining in the current window.
- `X-RateLimit-Reset`: The timestamp when the window resets.

## Request Deduplication

In distributed systems, clients may retry requests that have already been processed (e.g., due to network timeouts). Deduplication ensures that non-idempotent operations (like payments) are processed only once.

### How it works
The `DedupLayer` checks for an `Idempotency-Key` header. If a request with the same key is seen within the TTL window, it returns `409 Conflict`.

### Usage

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::dedup::DedupLayer;
use std::time::Duration;

fn main() {
    let app = RustApi::new()
        .layer(
            DedupLayer::new()
                .header_name("X-Idempotency-Key") // Optional: Custom header name
                .ttl(Duration::from_secs(300))    // 5 minutes TTL
        )
        .route("/payments", post(payment_handler));

    // ... run app
}
```

Clients should generate a unique UUID for each operation and send it in the `Idempotency-Key` header.

## Response Caching

Caching can significantly reduce load on your servers by serving stored responses for identical requests.

### How it works
The `CacheLayer` stores successful responses in memory based on the request method and URI. Subsequent requests are served from the cache until the TTL expires.

### Usage

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::cache::CacheLayer;
use std::time::Duration;

fn main() {
    let app = RustApi::new()
        .layer(
            CacheLayer::new()
                .ttl(Duration::from_secs(60)) // Cache for 60 seconds
                .add_method("GET")            // Cache GET requests
                .add_method("HEAD")           // Cache HEAD requests
        )
        .route("/heavy-computation", get(heavy_handler));

    // ... run app
}
```

Cached responses include an `X-Cache: HIT` header. Original responses have `X-Cache: MISS`.

## Combining Middleware

You can combine these layers to create a robust defense-in-depth strategy.

```rust
let app = RustApi::new()
    // 1. Rate Limit (Outer): Reject excessive traffic first
    .layer(RateLimitLayer::new(1000, Duration::from_secs(60)))

    // 2. Deduplication: Prevent double-processing
    .layer(DedupLayer::new())

    // 3. Cache: Serve static/computed content quickly
    .layer(CacheLayer::new().ttl(Duration::from_secs(30)))

    .route("/", get(handler));
```

> **Note**: Order matters! Placing Rate Limit first saves resources by rejecting requests before they hit the cache or application logic.
