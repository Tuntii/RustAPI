<div align="center">
  <img src="https://raw.githubusercontent.com/Tuntii/RustAPI/refs/heads/main/assets/logo.jpg" alt="RustAPI Logo" width="200" height="200" />
  <h1>RustAPI</h1>
  <p>
    <strong>The Ergonomic Web Framework for Rust.</strong><br>
    Built for Developers, Optimised for Production.
  </p>

  [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
  [![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
  [![Status](https://img.shields.io/badge/status-active-green.svg)](https://github.com/RustAPI/RustAPI)
</div>

<br />

## 🚀 Vision

**RustAPI** brings the developer experience (DX) of modern frameworks like **FastAPI** to the **Rust** ecosystem.

We believe that writing high-performance, type-safe web APIs in Rust shouldn't require fighting with complex trait bounds or massive boilerplate. RustAPI provides a polished, battery-included experience where:

*   **API Design is First-Class**: Define your schema, and let the framework handle Validation and OpenAPI documentation automatically.
*   **The Engine is Abstracted**: We rely on industry standards like `tokio`, `hyper`, and `matchit` internally, but we expose a stable, user-centric API. This means we can upgrade the engine without breaking your code.
*   **Zero Boilerplate**: Extractors and macros do the heavy lifting.

## ✨ Features

- **⚡ Fast & Async**: Built on top of `tokio` and `hyper` 1.0.
- **🛡️ Type-Safe**: Request/Response bodies are strictly typed using generic extractors (`Json`, `Query`, `Path`).
- **📝 Automatic OpenAPI**: Your code *is* your documentation. Swagger UI is served at `/docs` out of the box.
- **✅ Built-in Validation**: Add `#[validate(email)]` to your structs and get automatic 422 error handling.
- **🧩 Intuitive Routing**: Radix-tree based routing with simple macros `#[rustapi::get]`, `#[rustapi::post]`.
- **🔋 Batteries Included**: Middleware, JWT auth, CORS, rate limiting, and configuration management.
- **🔐 Security First**: JWT authentication, CORS middleware, and IP-based rate limiting out of the box.
- **⚙️ Configuration**: Environment-based config with `.env` file support and typed config extraction.

## 📦 Quick Start

Add `rustapi-rs` to your `Cargo.toml`.

```toml
[dependencies]
rustapi-rs = "0.1"

# Optional features
# rustapi-rs = { version = "0.1", features = ["jwt", "cors", "rate-limit"] }
```

```rust
use rustapi_rs::prelude::*;

/// Define your response schema
#[derive(Serialize, Schema)]
struct HelloResponse {
    message: String,
}

/// Define an endpoint
#[rustapi::get("/")]
#[rustapi::tag("General")]
#[rustapi::summary("Hello World Endpoint")]
async fn hello() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello from RustAPI!".to_string(),
    })
}

/// Run the server
#[rustapi::main]
async fn main() -> Result<()> {
    RustApi::new()
        .mount_route(hello_route()) // Auto-generated route handler
        .docs("/docs")              // Enable Swagger UI
        .run("127.0.0.1:8080")
        .await
}
```

Visit `http://127.0.0.1:8080/docs` to see your interactive API documentation!

## 🔐 Optional Features

RustAPI provides optional features to keep your binary size minimal:

| Feature | Description |
|---------|-------------|
| `jwt` | JWT authentication middleware and `AuthUser<T>` extractor |
| `cors` | CORS middleware with builder pattern configuration |
| `rate-limit` | IP-based rate limiting middleware |
| `config` | Configuration management with `.env` file support |
| `cookies` | Cookie parsing extractor |
| `sqlx` | SQLx database error conversion to ApiError |
| `extras` | Meta feature enabling jwt, cors, and rate-limit |
| `full` | All optional features enabled |

### JWT Authentication Example

```rust
use rustapi_rs::prelude::*;

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    exp: u64,
}

async fn protected(AuthUser(claims): AuthUser<Claims>) -> Json<String> {
    Json(format!("Hello, {}!", claims.sub))
}

#[tokio::main]
async fn main() -> Result<()> {
    RustApi::new()
        .with_middleware(JwtLayer::<Claims>::new("your-secret-key"))
        .route("/protected", get(protected))
        .run("127.0.0.1:8080")
        .await
}
```

### CORS Configuration Example

```rust
use rustapi_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let cors = CorsLayer::new()
        .allow_origins(["https://example.com"])
        .allow_methods([Method::GET, Method::POST])
        .allow_credentials(true);

    RustApi::new()
        .with_middleware(cors)
        .route("/api", get(handler))
        .run("127.0.0.1:8080")
        .await
}
```

### Rate Limiting Example

```rust
use rustapi_rs::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let rate_limit = RateLimitLayer::new(100, Duration::from_secs(60)); // 100 req/min

    RustApi::new()
        .with_middleware(rate_limit)
        .route("/api", get(handler))
        .run("127.0.0.1:8080")
        .await
}
```

## 🏗️ Architecture

RustAPI follows a **Facade Architecture** to ensure long-term stability:

*   **`rustapi-rs`**: The public-facing crate. It re-exports carefully selected types and traits to provide a clean surface.
*   **`rustapi-core`**: The internal engine. Handles the HTTP protocol, routing logic, and glue code.
*   **`rustapi-macros`**: Powers the ergonomic attributes like `#[rustapi::main]` and `#[rustapi::get]`.
*   **`rustapi-openapi` / `rustapi-validate`**: Specialized crates that wrap external ecosystems (`utoipa`, `validator`) into our consistent API.

## 🗺️ Roadmap

- [x] **Phase 1: MVP**: Core routing, extractors, and server.
- [x] **Phase 2: Validation & OpenAPI**: Auto-docs, strict validation, and metadata.
- [x] **Phase 3: Batteries Included**: Authentication (JWT), CORS, Rate Limiting, Middleware, and Configuration.
- [ ] **Phase 4: v1.0 Polish**: Advanced ergonomics, CLI tool, and production hardening.


## 📄 License

This project is licensed under either of

*   Apache License, Version 2.0
*   MIT license

at your option.
