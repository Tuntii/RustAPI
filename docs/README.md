# RustAPI Documentation

Welcome to the RustAPI documentation!

## Quick Links

| Document | Description |
|----------|-------------|
| [Getting Started](GETTING_STARTED.md) | Build your first API in 5 minutes |
| [Features](FEATURES.md) | Complete feature reference |
| [Philosophy](PHILOSOPHY.md) | Design principles and decisions |
| [Architecture](ARCHITECTURE.md) | Internal structure deep dive |

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

See the [examples](../examples/) directory:
- `hello-world` — Minimal example
- `crud-api` — Full CRUD operations
- `auth-api` — JWT authentication
- `toon-api` — LLM-optimized responses
- `proof-of-concept` — Complete feature showcase

## License

MIT or Apache-2.0, at your option.
