<div align="center">
  <img src="https://raw.githubusercontent.com/Tuntii/RustAPI/refs/heads/main/assets/logo.jpg" alt="RustAPI" width="200" />
  
  # RustAPI
  
  A high-performance, ergonomic web framework for Rust with native AI/LLM support.

  [![Crates.io](https://img.shields.io/crates/v/rustapi-rs.svg)](https://crates.io/crates/rustapi-rs)
  [![Docs](https://img.shields.io/badge/docs-cookbook-brightgreen)](docs/cookbook/src/SUMMARY.md)
  [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
  [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Tuntii/RustAPI)
</div>

---

## Overview

RustAPI is a Rust web framework built on **hyper 1.x** and **tokio**, designed for minimal boilerplate while retaining full control over performance. It uses a **facade architecture** (`rustapi-rs`) that shields user code from internal crate changes, keeping the public API stable as internals evolve.

Key design goals:
- **Ergonomic handler signatures** inspired by FastAPI and Axum
- **Native TOON format** for token-efficient LLM responses
- **Auto-discovery** of routes via procedural macros and link-time registration
- **Three-tier request execution** (ultra fast / fast / full) to minimize overhead

## What Sets RustAPI Apart

### Three-Tier Request Execution

Unlike other Rust frameworks that always run the full middleware chain, RustAPI dynamically selects the cheapest execution path per request:

| Path | When | Overhead |
|:-----|:-----|:---------|
| **Ultra Fast** | No middleware, no interceptors | Zero Arc cloning, direct handler call |
| **Fast** | Interceptors only, no middleware layers | Interceptor functions only |
| **Full** | Middleware layers present | Complete `LayerStack` execution |

This means a simple `GET /health` endpoint with no middleware runs at near-zero overhead, while other endpoints on the same server can use JWT, CORS, and rate limiting through the full path.

### Facade Architecture with Contract Enforcement

User code imports only from `rustapi-rs`. Internal crates (`rustapi-core`, `rustapi-macros`, etc.) can be refactored freely without breaking user code. The public API surface is tracked via committed `cargo public-api` snapshots, and CI enforces labeling rules (`breaking` / `feature`) on any PR that changes them.

### Link-Time Auto-Discovery

Routes annotated with `#[rustapi_rs::get("/...")]` are registered to a `linkme` distributed slice at link time — no manual route registration or inventory macros needed. `RustApi::auto()` collects them and builds a `BTreeMap`-ordered radix tree router via `matchit`.

### TOON: Token-Oriented Object Notation

A compact serialization format that reduces token counts by **50-58%** compared to JSON. `Toon<T>` is a drop-in replacement for `Json<T>`. `LlmResponse<T>` performs automatic content negotiation based on `Accept` headers and adds headers like `X-Token-Count-JSON`, `X-Token-Count-TOON`, and `X-Token-Savings` for observability.

### Built-in Resilience Primitives

RustAPI ships circuit breaker and retry middleware as first-class features, not third-party crate bolt-ons:

- **Circuit Breaker** (`CircuitBreakerLayer`): Fault tolerance with open/half-open/closed states
- **Retry** with exponential backoff
- **Rate Limiting** (IP-based, per-route)
- **Body Limit** with configurable max size (default 1 MB)
- **Health Probes** via `.health_endpoints()` for `/health`, `/ready`, and `/live`

### Environment-Aware Error Masking

All error responses include a unique `error_id` (`err_{uuid}`) for log correlation. In production (`RUSTAPI_ENV=production`), 5xx error details are automatically masked to `"An internal error occurred"` while validation errors (4xx) pass through intact.

### Request Replay & Time-Travel Debugging

Record and replay HTTP request/response pairs for production debugging:

```rust
RustApi::new()
    .layer(ReplayLayer::new(store, config))
    .run("0.0.0.0:8080").await;
```

```sh
cargo rustapi replay list
cargo rustapi replay run <id> --target http://localhost:8080
cargo rustapi replay diff <id> --target http://staging
```

- Middleware-based recording; no application code changes
- Sensitive header redaction; disabled by default
- In-memory (dev) or filesystem (production) storage with TTL
- `ReplayClient` for programmatic test automation

### Dual-Stack HTTP/1.1 + HTTP/3

Run HTTP/1.1 (TCP) and HTTP/3 (QUIC/UDP) simultaneously on the same server. Enable with the `core-http3` feature flag.

### Native OpenAPI 3.1

`#[derive(Schema)]` generates OpenAPI schemas at compile time. `RustApi::auto()` assembles the full spec with reference integrity validation. Swagger UI is served at `/docs` by default. No external code generators or YAML files needed.

### Async Validation with Application State

`AsyncValidatedJson<T>` can access application state (e.g., database connections) during validation. The extractor clones `ValidationContext` from request state, enabling rules like "username must be unique" at the validation layer.

### Background Jobs

`rustapi-jobs` provides an async job queue with three backends (Memory, Redis, Postgres), retry with exponential backoff, dead letter queues, and scheduled execution.

### Side-by-Side gRPC + HTTP

`rustapi-grpc` enables running Tonic-based gRPC services alongside RustAPI HTTP handlers in the same process via `run_rustapi_and_grpc`.

### Additional Built-in Capabilities

| Capability | Notes |
|:-----------|:------|
| WebSocket with permessage-deflate | Full compression negotiation via `protocol-ws` |
| Server-Sent Events (SSE) | `SseEvent` with id, event type, retry fields |
| Tera template rendering | `View<T>` response type via `protocol-view` |
| JWT authentication | `AuthUser<T>` extractor + `JwtLayer` |
| CORS | `CorsLayer` with builder pattern |
| `simd-json` acceleration | 2-4x faster JSON parsing via `core-simd-json` feature |
| In-memory `TestClient` | Executes the full middleware stack without network I/O |
| `MockServer` | Expectation-based mock with explicit `verify()` |
| `cargo rustapi new` | Interactive project scaffolding with feature selection |

## Comparison

| Feature | RustAPI | Actix-web | Axum | FastAPI (Python) |
|:--------|:-------:|:---------:|:----:|:----------------:|
| Performance | See benchmark source | Workload-dependent | Workload-dependent | Workload-dependent |
| Ergonomics | High | Low | Medium | High |
| AI/LLM native format (TOON) | Yes | No | No | No |
| Request replay / time-travel debug | Built-in | No | No | 3rd-party |
| Circuit breaker / retry | Built-in | 3rd-party | 3rd-party | 3rd-party |
| Adaptive execution paths | 3-tier | No | No | N/A |
| OpenAPI from code | Compile-time derive | 3rd-party | 3rd-party | Built-in |
| HTTP/3 (QUIC) | Built-in | No | 3rd-party | No |
| Background jobs | Built-in | 3rd-party | 3rd-party | 3rd-party |
| API stability model | Facade + CI contract | Direct | Direct | Stable |

Current benchmark methodology and canonical published performance claims live in [`docs/PERFORMANCE_BENCHMARKS.md`](docs/PERFORMANCE_BENCHMARKS.md). Historical point-in-time numbers in older release notes should not be treated as the current baseline unless they are linked from that document.

## Quick Start

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize, Schema)]
struct Message { text: String }

#[rustapi_rs::get("/hello/{name}")]
async fn hello(Path(name): Path<String>) -> Json<Message> {
    Json(Message { text: format!("Hello, {}!", name) })
}

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
  RustApi::auto().run("127.0.0.1:8080").await
}
```

`RustApi::auto()` collects all macro-annotated handlers, generates OpenAPI documentation (served at `/docs`), and starts a multi-threaded tokio runtime.

For production deployments, you can enable standard probe endpoints without writing handlers manually:

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let health = HealthCheckBuilder::new(true)
        .add_check("database", || async { HealthStatus::healthy() })
        .build();

    RustApi::auto()
        .with_health_check(health)
        .run("127.0.0.1:8080")
        .await
}
```

This registers:
- `/health` — aggregate dependency health
- `/ready` — readiness probe (`503` when dependencies are unhealthy)
- `/live` — lightweight liveness probe

Or use a single production baseline preset:

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
  RustApi::auto()
    .production_defaults("users-api")
    .run("127.0.0.1:8080")
    .await
}
```

`production_defaults()` enables request IDs, tracing spans, and standard probe endpoints in one call.

You can shorten the macro prefix by renaming the crate:

```toml
[dependencies]
api = { package = "rustapi-rs", version = "0.1.335" }
```

```rust
use api::prelude::*;

#[api::get("/users")]
async fn list_users() -> &'static str { "ok" }
```

## Feature Flags

Features are organized into three namespaces:

| Namespace | Purpose | Examples |
|:----------|:--------|:--------|
| `core-*` | Core framework capabilities | `core-openapi`, `core-tracing`, `core-http3` |
| `protocol-*` | Optional protocol support | `protocol-toon`, `protocol-ws`, `protocol-view`, `protocol-grpc` |
| `extras-*` | Production middleware | `extras-jwt`, `extras-cors`, `extras-rate-limit`, `extras-replay` |

Meta features: `core` (default), `protocol-all`, `extras-all`, `full`.

## Recent Changes (v0.1.335)

- Dual-stack runtime: simultaneous HTTP/1.1 (TCP) and HTTP/3 (QUIC/UDP)
- WebSocket permessage-deflate compression
- Improved OpenAPI reference integrity and validation documentation
- Async validation with application state integration
- `rustapi-grpc` crate: optional Tonic/Prost-based gRPC alongside HTTP (`run_rustapi_and_grpc`)
- `cargo rustapi new` now includes `grpc` in interactive feature selection

## Roadmap (February 2026)

- [x] Visual status page: automatic health dashboard
- [x] gRPC integration via `rustapi-grpc`
- [x] Distributed tracing: OpenTelemetry integration
- [ ] RustAPI Cloud: managed deployment to major cloud providers

## Documentation

Detailed architecture, recipes, and guides are in the [Cookbook](docs/cookbook/src/SUMMARY.md):

- [System Architecture](docs/cookbook/src/architecture/system_overview.md)
- [Performance Benchmarks](docs/cookbook/src/concepts/performance.md)
- [gRPC Integration Guide](docs/cookbook/src/crates/rustapi_grpc.md)
- [Recommended Production Baseline](docs/PRODUCTION_BASELINE.md)
- [Production Checklist](docs/PRODUCTION_CHECKLIST.md)
- [Examples](crates/rustapi-rs/examples/)

---

<div align="center">
  <sub>Built by Tunti35.</sub>
</div>
