<div align="center">
  <img src="https://raw.githubusercontent.com/Tuntii/RustAPI/refs/heads/main/assets/logo.jpg" alt="RustAPI Logo" width="200" height="200" />

  <h1>RustAPI</h1>
  <p>
    <strong>The Ergonomic Web Framework for Rust.</strong><br>
    Built for Developers, Optimized for Production.
  </p>

  [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
  [![Crates.io](https://img.shields.io/crates/v/rustapi-rs.svg)](https://crates.io/crates/rustapi-rs)
  [![Docs.rs](https://docs.rs/rustapi-rs/badge.svg)](https://docs.rs/rustapi-rs)
</div>

<br />

## 🚀 Vision

**RustAPI** brings the developer experience (DX) of modern frameworks like **FastAPI** to the **Rust** ecosystem.

We believe that writing high-performance, type-safe web APIs in Rust shouldn't require fighting with complex trait bounds or massive boilerplate. RustAPI provides a polished, battery-included experience where:

*   **API Design is First-Class**: Define your schema, and let the framework handle Validation and OpenAPI documentation automatically.
*   **The Engine is Abstracted**: We rely on industry standards like `tokio`, `hyper`, and `matchit` internally, but we expose a stable, user-centric API.
*   **Zero Boilerplate**: Extractors and macros do the heavy lifting.

## ✨ Features

- **⚡ Fast & Async**: Built on top of `tokio` and `hyper` 1.0.
- **🛡️ Type-Safe**: Request/Response bodies are strictly typed using generic extractors (`Json`, `Query`, `Path`).
- **📝 Auto-Docs**: Generates **OpenAPI 3.0** specifications and serves **Swagger UI** automatically.
- **✅ Validation**: Declarative validation using `#[derive(Validate)]`.
- **🔌 Batteries Included**: 
    - **Authentication**: JWT support.
    - **Database**: SQLx integration.
    - **WebSockets**: Real-time communication.
    - **Templating**: Tera view engine.
    - **Jobs**: Background task processing (Redis/Postgres).

## 📦 Quick Start

Add `rustapi-rs` to your `Cargo.toml`.

```toml
[dependencies]
rustapi-rs = { version = "0.1", features = ["full"] }
```

### The "Hello World"

```rust
use rustapi_rs::prelude::*;

/// Define your response schema
#[derive(Serialize, Schema)]
struct HelloResponse {
    message: String,
}

/// Define an endpoint
#[rustapi_rs::get("/")]
#[rustapi_rs::tag("General")]
#[rustapi_rs::summary("Hello World Endpoint")]
async fn hello() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello from RustAPI!".to_string(),
    })
}

/// Run the server
#[rustapi_rs::main]
async fn main() -> Result<()> {
    RustApi::new()
        .api_name("My Awesome API")
        .api_version("1.0.0")
        .mount_route(hello_route()) // Auto-generated route handler
        .docs("/docs")              // Enable Swagger UI
        .run("127.0.0.1:8080")
        .await
}
```

Visit `http://127.0.0.1:8080/docs` to see your interactive API documentation!

## 🗺️ Architecture

RustAPI follows a **Facade Architecture**:

*   **`rustapi-rs`**: The public-facing entry point. Always import from here.
*   **`rustapi-core`**: The internal engine (Hyper/Tower).
*   **`rustapi-macros`**: Procedural macros (`#[get]`, `#[main]`).
*   **`cargo-rustapi`**: The CLI tool for scaffolding projects.

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
