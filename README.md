<div align="center">
  <img src="https://raw.githubusercontent.com/Tuntii/RustAPI/refs/heads/main/assets/logo.jpg" alt="RustAPI" width="180" />
  
  # RustAPI
  
  **The power of Rust. Modern DX. LLM-ready.**

  [![Crates.io](https://img.shields.io/crates/v/rustapi-rs.svg)](https://crates.io/crates/rustapi-rs)
  [![Docs.rs](https://img.shields.io/docsrs/rustapi-rs)](https://docs.rs/rustapi-rs)
  [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
</div>

---

## Vision

RustAPI redefines **API development for the AI era**.

We combine Rust's performance and safety with FastAPI's ergonomics. Write type-safe, production-ready APIs without fighting trait bounds. **MCP servers**, **LLM integrations**, or classic REST APIs — one framework for all.

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

5 lines of code. Auto-generated OpenAPI docs. Production-ready.

---

## Quick Start

```toml
[dependencies]
rustapi-rs = "0.1.3"
```

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize, Schema)]
struct User { id: u64, name: String }

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<u64>) -> Json<User> {
    Json(User { id, name: "Tunahan".into() })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Zero config: all `#[rustapi_rs::get/post/..]` routes are auto-registered.
    // Swagger UI is enabled at /docs by default (when built with the `swagger-ui` feature).
    RustApi::auto().run("127.0.0.1:8080").await
}
```

`http://localhost:8080/docs` → Swagger UI ready.

---

## Features

| Feature | Description |
|---------|-------------|
| **Type-Safe Extractors** | `Json<T>`, `Query<T>`, `Path<T>` — compile-time guarantees |
| **Zero-Config Routing** | Macro-decorated routes auto-register at startup (`RustApi::auto()`) |
| **Auto OpenAPI** | Your code = your docs. `/docs` endpoint out of the box |
| **Validation** | `#[validate(email)]` → automatic 422 responses |
| **JWT Auth** | One-line auth with `AuthUser<T>` extractor |
| **CORS & Rate Limit** | Production-ready middleware |
| **TOON Format** | **50-58% token savings** for LLMs |

### Optional Features

```toml
rustapi-rs = { version = "0.1.3", features = ["jwt", "cors", "toon"] }
```

- `jwt` — JWT authentication
- `cors` — CORS middleware  
- `rate-limit` — IP-based rate limiting
- `toon` — LLM-optimized responses
- `full` — Everything included

---

## Examples

All examples in this repository are written in the Phase 6 “zero-config” style.

```bash
cargo run -p hello-world
cargo run -p crud-api
cargo run -p auth-api
cargo run -p sqlx-crud
cargo run -p toon-api
cargo run -p proof-of-concept
```

---

## 🤖 LLM-Optimized: TOON Format

RustAPI is built for **AI-powered APIs**.

**TOON** (Token-Oriented Object Notation) uses **50-58% fewer tokens** than JSON. Ideal for MCP servers, AI agents, and LLM integrations.

```rust
use rustapi_rs::toon::{Toon, LlmResponse, AcceptHeader};

// Direct TOON response
#[rustapi::get("/ai/users")]
async fn ai_users() -> Toon<UsersResponse> {
    Toon(get_users())
}

// Content negotiation: JSON or TOON based on Accept header
#[rustapi::get("/users")]
async fn users(accept: AcceptHeader) -> LlmResponse<UsersResponse> {
    LlmResponse::new(get_users(), accept.preferred)
}
// Headers: X-Token-Count-JSON, X-Token-Count-TOON, X-Token-Savings
```

**Why TOON?**
- Compatible with Claude, GPT-4, Gemini — all major LLMs
- Cut your token costs in half
- Optimized for MCP (Model Context Protocol) servers

---

## Architecture

RustAPI follows a **Facade Architecture** — a stable public API that shields you from internal changes.

### System Overview

```mermaid
graph TB
    subgraph Client["🌐 Client Layer"]
        HTTP[HTTP Request]
        LLM[LLM/AI Agent]
        MCP[MCP Client]
    end

    subgraph Public["📦 rustapi-rs (Public Facade)"]
        direction TB
        Prelude[prelude::*]
        Macros["#[rustapi::get/post]<br>#[rustapi::main]"]
        Types[Json, Query, Path, Form]
    end

    subgraph Core["⚙️ rustapi-core (Engine)"]
        direction TB
        Router[Radix Router<br>matchit]
        Extract[Extractors<br>FromRequest trait]
        MW[Middleware Stack<br>Tower-like layers]
        Resp[Response Builder<br>IntoResponse trait]
    end

    subgraph Extensions["🔌 Extension Crates"]
        direction LR
        OpenAPI["rustapi-openapi<br>Swagger/Docs"]
        Validate["rustapi-validate<br>Request Validation"]
        Toon["rustapi-toon<br>LLM Optimization"]
        Extras["rustapi-extras<br>JWT/CORS/RateLimit"]
    end

    subgraph Foundation["🏗️ Foundation Layer"]
        direction LR
        Tokio[tokio<br>Async Runtime]
        Hyper[hyper 1.0<br>HTTP Protocol]
        Serde[serde<br>Serialization]
    end

    HTTP --> Public
    LLM --> Public
    MCP --> Public
    Public --> Core
    Core --> Extensions
    Extensions --> Foundation
    Core --> Foundation
```

### Request Flow

```mermaid
sequenceDiagram
    participant C as Client
    participant R as Router
    participant M as Middleware
    participant E as Extractors
    participant H as Handler
    participant S as Serializer

    C->>R: HTTP Request
    R->>R: Match route (radix tree)
    R->>M: Pass to middleware stack
    
    loop Each Middleware
        M->>M: Process (JWT, CORS, RateLimit)
    end
    
    M->>E: Extract parameters
    E->>E: Json<T>, Path<T>, Query<T>
    E->>E: Validate with #[validate]
    
    alt Validation Failed
        E-->>C: 422 Unprocessable Entity
    else Validation OK
        E->>H: Call async handler
        H->>S: Return response type
        
        alt TOON Enabled
            S->>S: Check Accept header
            S->>S: Serialize as TOON/JSON
            S->>S: Add token count headers
        else Standard
            S->>S: Serialize as JSON
        end
        
        S-->>C: HTTP Response
    end
```

### Crate Dependency Graph

```mermaid
graph BT
    subgraph User["Your Application"]
        App[main.rs]
    end

    subgraph Facade["Single Import"]
        RS[rustapi-rs]
    end

    subgraph Internal["Internal Crates"]
        Core[rustapi-core]
        Macros[rustapi-macros]
        OpenAPI[rustapi-openapi]
        Validate[rustapi-validate]
        Toon[rustapi-toon]
        Extras[rustapi-extras]
    end

    subgraph External["External Dependencies"]
        Tokio[tokio]
        Hyper[hyper]
        Serde[serde]
        Utoipa[utoipa]
        Validator[validator]
    end

    App --> RS
    RS --> Core
    RS --> Macros
    RS --> OpenAPI
    RS --> Validate
    RS -.->|optional| Toon
    RS -.->|optional| Extras
    
    Core --> Tokio
    Core --> Hyper
    Core --> Serde
    OpenAPI --> Utoipa
    Validate --> Validator
    Toon --> Serde

    style RS fill:#e1f5fe
    style App fill:#c8e6c9
```

### Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Single Entry Point** | `use rustapi_rs::prelude::*` imports everything you need |
| **Zero Boilerplate** | Macros generate routing, OpenAPI specs, and validation |
| **Compile-Time Safety** | Generic extractors catch type errors at compile time |
| **Opt-in Complexity** | Features like JWT, TOON are behind feature flags |
| **Engine Abstraction** | Internal hyper/tokio upgrades don't break your code |

### Crate Responsibilities

| Crate | Role |
|-------|------|
| `rustapi-rs` | Public facade — single `use` for everything |
| `rustapi-core` | HTTP engine, routing, extractors, response handling |
| `rustapi-macros` | Procedural macros: `#[rustapi::get]`, `#[rustapi::main]` |
| `rustapi-openapi` | Swagger UI generation, OpenAPI 3.0 spec |
| `rustapi-validate` | Request body/query validation via `#[validate]` |
| `rustapi-toon` | TOON format serializer, content negotiation, LLM headers |
| `rustapi-extras` | JWT auth, CORS, rate limiting middleware |

---

## Roadmap

- [x] Core framework (routing, extractors, server)
- [x] OpenAPI & Validation
- [x] JWT, CORS, Rate Limiting
- [x] TOON format & LLM optimization
- [ ] *Coming soon...*

---

## License

MIT or Apache-2.0, at your option.
