# RustAPI Documentation

Welcome to the RustAPI documentation!

## Quick Links

| Document | Description |
|----------|-------------|
| [Getting Started](GETTING_STARTED.md) | Build your first API in 5 minutes |
| [Features](FEATURES.md) | Complete feature reference |
| [Philosophy](PHILOSOPHY.md) | Design principles and decisions |
| [Architecture](ARCHITECTURE.md) | Internal structure deep dive |
| [Performance Benchmarks](PERFORMANCE_BENCHMARKS.md) | Authoritative source for benchmark methodology and published claims |
| [Recommended Production Baseline](PRODUCTION_BASELINE.md) | Opinionated starting point for production services |
| [Production Checklist](PRODUCTION_CHECKLIST.md) | Rollout-ready operational checklist |

## What is RustAPI?

RustAPI is an ergonomic web framework for Rust, inspired by FastAPI's developer experience. It combines Rust's performance and safety with modern DX.

**Key Features:**
- 🎯 5-line APIs — Minimal boilerplate
- 🛡️ Type Safety — Compile-time guarantees
- 📖 Auto Documentation — Swagger UI out of the box
- 🤖 LLM-Ready — TOON format saves 50-58% tokens
- 🔒 Production Ready — JWT, CORS, rate limiting included

## Philosophy

> *"API surface is ours, engines can change."*

RustAPI provides a stable, ergonomic public API. Internal dependencies (`hyper`, `tokio`, `validator`) are implementation details that can be upgraded without breaking your code.
The stable contract lives in `rustapi-rs`; internal crates are not compatibility targets.

Feature taxonomy:
- `core-*` for framework core behavior.
- `protocol-*` for optional protocol integrations.
- `extras-*` for optional production middleware/integrations.

## Getting Started

```toml
[dependencies]
rustapi-rs = "0.1.335"
```

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/hello/{name}")]
async fn hello(Path(name): Path<String>) -> Json<Message> {
    Json(Message { greeting: format!("Hello, {name}!") })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::auto().run("0.0.0.0:8080").await
}
```

Visit `http://localhost:8080/docs` for auto-generated Swagger UI.

## Examples

See [`crates/rustapi-rs/examples/README.md`](../crates/rustapi-rs/examples/README.md) for the current in-repository example index.

Current examples in this repository:
- `typed_path_poc` — Typed path registration and URI generation
- `status_demo` — Automatic status page demo with live traffic/error generation

## Production Guides

- [Recommended Production Baseline](PRODUCTION_BASELINE.md)
- [Production Checklist](PRODUCTION_CHECKLIST.md)
- [Cookbook: Graceful Shutdown](cookbook/src/recipes/graceful_shutdown.md)
- [Cookbook: Deployment](cookbook/src/recipes/deployment.md)
- [Cookbook: Observability](cookbook/src/recipes/observability.md)

## License

MIT or Apache-2.0, at your option.
