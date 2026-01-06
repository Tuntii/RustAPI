<div align="center">
  <img src="https://raw.githubusercontent.com/Tuntii/RustAPI/refs/heads/main/assets/logo.jpg" alt="RustAPI" width="180" />
  
  # RustAPI
  
  **The power of Rust. Modern DX. LLM-ready.**

  [![Crates.io](https://img.shields.io/crates/v/rustapi-rs.svg)](https://crates.io/crates/rustapi-rs)
  [![Docs.rs](https://img.shields.io/docsrs/rustapi-rs)](https://docs.rs/rustapi-rs)
  [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
  [![Downloads](https://img.shields.io/crates/d/rustapi-rs)](https://crates.io/crates/rustapi-rs)
  [![Build Status](https://img.shields.io/github/actions/workflow/status/Tuntii/RustAPI/rust.yml)](https://github.com/Tuntii/RustAPI/actions)
  [![Stars](https://img.shields.io/github/stars/Tuntii/RustAPI?style=social)](https://github.com/Tuntii/RustAPI/stargazers)
  
  <a href="https://www.producthunt.com/products/rustapi?embed=true&utm_source=badge-featured&utm_medium=badge&utm_campaign=badge-rustapi" target="_blank"><img src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1057797&theme=dark&t=1767462180457" alt="RustAPI - A Rust API framework designed for AI-first development | Product Hunt" width="250" height="54" /></a>
</div>

---

## Vision

RustAPI redefines **API development for the AI era**.

We combine Rust's performance and safety with FastAPI's ergonomics. Write type-safe, production-ready APIs without fighting trait bounds. **MCP servers**, **LLM integrations**, or classic REST APIs — one framework for all.

---

## Philosophy

> *"API surface is ours, engines can change."*

RustAPI follows a **Facade Architecture** — a stable, ergonomic public API that shields you from internal complexity and breaking changes.

### Core Principles

| Principle | What It Means |
|-----------|---------------|
| **🎯 5-Line APIs** | A working REST endpoint in 5 lines. No ceremony. |
| **🛡️ Stable Surface** | Your code depends on `rustapi-rs`. Internal crates (`hyper`, `tokio`, `validator`) are implementation details. |
| **🔄 Engines Change** | We can swap `hyper` for `h3`, upgrade `tokio`, or replace `validator` — your code stays the same. |
| **🎁 Batteries Included** | JWT, CORS, Rate Limiting, OpenAPI — all built-in, all optional via feature flags. |
| **🤖 LLM-First** | TOON format, token counting headers, MCP-ready. Built for the AI era. |

### Why This Matters

```
┌─────────────────────────────────────────────────────────────┐
│                    Your Application                          │
│                   use rustapi_rs::prelude::*                 │
├─────────────────────────────────────────────────────────────┤
│                     rustapi-rs (Facade)                      │
│              Stable API ── Never Breaks                      │
├───────────────┬───────────────┬───────────────┬─────────────┤
│ rustapi-core  │ rustapi-toon  │ rustapi-extras│ ...         │
│   (hyper)     │   (serde)     │    (jwt)      │             │
├───────────────┴───────────────┴───────────────┴─────────────┤
│              Foundation: tokio, serde, hyper                 │
│              ↑ Can be upgraded/swapped internally            │
└─────────────────────────────────────────────────────────────┘
```

**Internal upgrades don't break your code.** When `hyper 2.0` releases, we update `rustapi-core`. Your `RustApi::new()` keeps working.

📚 Read more: [docs/PHILOSOPHY.md](docs/PHILOSOPHY.md) | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

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

## 🚀 Performance

RustAPI is **blazingly fast** — built on Tokio and Hyper 1.0, with zero-cost abstractions.

### Benchmarks

| Framework | Requests/sec | Latency (avg) | Memory |
|-----------|--------------|---------------|--------|
| **RustAPI** | **~185,000** | **~0.54ms** | **~8MB** |
| Actix-web | ~178,000 | ~0.56ms | ~10MB |
| Axum | ~165,000 | ~0.61ms | ~12MB |
| Rocket | ~95,000 | ~1.05ms | ~15MB |
| FastAPI (Python) | ~12,000 | ~8.3ms | ~45MB |

<details>
<summary>🔬 Test Configuration</summary>

- **Hardware**: Intel i7-12700K, 32GB RAM
- **Method**: `wrk -t12 -c400 -d30s http://127.0.0.1:8080/api/users`
- **Scenario**: JSON serialization of 100 user objects
- **Build**: `cargo build --release`

Results may vary based on hardware and workload. Run your own benchmarks:
```bash
cd benches
./run_benchmarks.ps1
```
</details>

### Why So Fast?

- ⚡ **Zero-copy parsing** — Direct memory access for path/query params
- 🔄 **Async-first** — Tokio runtime handles 100K+ concurrent connections
- 📦 **Smart caching** — Route matching cached via radix tree (matchit)
- 🎯 **No dynamic dispatch** — All extractors resolved at compile time

---

## Quick Start

```toml
[dependencies]
rustapi-rs = "0.1.4"
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
| **Type-Safe Extractors** | `Json<T>`, `Query<T>`, `Path<T>`, `WebSocket` — compile-time guarantees |
| **Zero-Config Routing** | Macro-decorated routes auto-register at startup (`RustApi::auto()`) |
| **Auto OpenAPI** | Your code = your docs. `/docs` endpoint out of the box |
| **Validation** | `#[validate(email)]` → automatic 422 responses |
| **JWT Auth** | One-line auth with `AuthUser<T>` extractor |
| **CORS & Rate Limit** | Production-ready middleware |
| **TOON Format** | **50-58% token savings** for LLMs |
| **WebSocket** | Real-time bidirectional communication with broadcast support |
| **Template Engine** | Server-side HTML rendering with Tera templates |
| **CLI Tool** | `cargo-rustapi` for project scaffolding |

### Optional Features

```toml
rustapi-rs = { version = "0.1.4", features = ["jwt", "cors", "toon", "ws", "view"] }
```

- `jwt` — JWT authentication
- `cors` — CORS middleware  
- `rate-limit` — IP-based rate limiting
- `toon` — LLM-optimized responses
- `ws` — WebSocket support
- `view` — Template engine (Tera)
- `full` — Everything included

---

## Examples

All examples in this repository are written in the Phase 6 “zero-config” style.

```bash
cargo run -p hello-world         # 5-line hello world
cargo run -p crud-api            # Full CRUD with validation
cargo run -p auth-api            # JWT authentication
cargo run -p sqlx-crud           # Database integration (PostgreSQL)
cargo run -p toon-api            # TOON format for LLMs
cargo run -p websocket           # Real-time WebSocket chat
cargo run -p templates           # Server-side rendering with Tera
cargo run -p mcp-server          # Model Context Protocol server
cargo run -p rate-limit-demo     # Rate limiting concepts
cargo run -p microservices       # Service-to-service communication
cargo run -p middleware-chain    # Middleware patterns
# cargo run -p graphql-api       # GraphQL (coming soon)
```

### 📚 Example Categories

<details>
<summary><b>🌟 Getting Started</b></summary>

- **[hello-world](examples/hello-world/)** — Minimal 5-line API
- **[crud-api](examples/crud-api/)** — Complete CRUD with in-memory storage
- **[proof-of-concept](examples/proof-of-concept/)** — Feature showcase

</details>

<details>
<summary><b>🔐 Authentication & Security</b></summary>

- **[auth-api](examples/auth-api/)** — JWT authentication & authorization
- **[rate-limit-demo](examples/rate-limit-demo/)** — IP-based rate limiting
- **[middleware-chain](examples/middleware-chain/)** — Custom auth middleware

</details>

<details>
<summary><b>🗄️ Database Integration</b></summary>

- **[sqlx-crud](examples/sqlx-crud/)** — PostgreSQL with SQLx
- **[database-pooling](examples/database-pooling/)** — Connection pool management
- **[redis-cache](examples/redis-cache/)** — Redis caching layer

</details>

<details>
<summary><b>🤖 AI & LLM</b></summary>

- **[toon-api](examples/toon-api/)** — TOON format for token optimization
- **[mcp-server](examples/mcp-server/)** — Model Context Protocol integration
- **[llm-streaming](examples/llm-streaming/)** — Streaming LLM responses

</details>

<details>
<summary><b>🌐 Real-time & Web</b></summary>

- **[websocket](examples/websocket/)** — WebSocket chat with broadcast
- **[templates](examples/templates/)** — Server-side HTML rendering
- **[sse-events](examples/sse-events/)** — Server-Sent Events

</details>

<details>
<summary><b>🏗️ Advanced Patterns</b></summary>

- **[graphql-api](examples/graphql-api/)** — GraphQL with async-graphql
- **[microservices](examples/microservices/)** — Multi-service architecture
- **[grpc-integration](examples/grpc-integration/)** — gRPC + REST hybrid

</details>

---

## 🔌 Real-time: WebSocket Support

RustAPI provides first-class WebSocket support for real-time applications.

```rust
use rustapi_rs::ws::{WebSocket, Message, Broadcast};

#[rustapi_rs::get("/ws")]
async fn websocket(ws: WebSocket) -> WebSocketUpgrade {
    ws.on_upgrade(handle_connection)
}

async fn handle_connection(mut stream: WebSocketStream) {
    while let Some(msg) = stream.recv().await {
        match msg {
            Message::Text(text) => {
                stream.send(Message::Text(format!("Echo: {}", text))).await.ok();
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
```

**Features:**
- Full WebSocket protocol support (text, binary, ping/pong)
- `Broadcast` channel for pub/sub patterns
- Seamless integration with RustAPI routing

---

## 🎨 Template Engine

Server-side HTML rendering with Tera templates.

```rust
use rustapi_rs::view::{Templates, View, ContextBuilder};

#[rustapi_rs::get("/")]
async fn home(templates: Templates) -> View<()> {
    View::new(&templates, "index.html", ())
}

#[rustapi_rs::get("/users/{id}")]
async fn user_page(templates: Templates, Path(id): Path<u64>) -> View<User> {
    let user = get_user(id);
    View::with_context(&templates, "user.html", user, |ctx| {
        ctx.insert("title", &format!("User: {}", user.name));
    })
}
```

**Features:**
- Tera template engine (Jinja2-like syntax)
- Type-safe context with `ContextBuilder`
- Template inheritance support
- Auto-escape HTML by default

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

## 🛠️ CLI Tool: cargo-rustapi

Scaffold new RustAPI projects with ease.

```bash
# Install the CLI
cargo install cargo-rustapi

# Create a new project
cargo rustapi new my-api

# Interactive mode
cargo rustapi new my-api --interactive
```

**Available Templates:**
- `minimal` — Basic RustAPI setup
- `api` — REST API with CRUD operations
- `web` — Full web app with templates and WebSocket
- `full` — Everything included

**Commands:**
- `cargo rustapi new <name>` — Create new project
- `cargo rustapi generate <type>` — Generate handlers, models, middleware
- `cargo rustapi docs` — Generate API documentation

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
        WsCrate["rustapi-ws<br>WebSocket Support"]
        ViewCrate["rustapi-view<br>Template Engine"]
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
        WS[rustapi-ws]
        View[rustapi-view]
    end

    subgraph External["External Dependencies"]
        Tokio[tokio]
        Hyper[hyper]
        Serde[serde]
        Utoipa[utoipa]
        Validator[validator]
        Tungstenite[tungstenite]
        Tera[tera]
    end

    App --> RS
    RS --> Core
    RS --> Macros
    RS --> OpenAPI
    RS --> Validate
    RS -.->|optional| Toon
    RS -.->|optional| Extras
    RS -.->|optional| WS
    RS -.->|optional| View
    
    Core --> Tokio
    Core --> Hyper
    Core --> Serde
    OpenAPI --> Utoipa
    Validate --> Validator
    Toon --> Serde
    WS --> Tungstenite
    View --> Tera

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
| `rustapi-ws` | WebSocket support with broadcast channels |
| `rustapi-view` | Template engine (Tera) for server-side rendering |

---

## Roadmap

- [x] Core framework (routing, extractors, server)
- [x] OpenAPI & Validation
- [x] JWT, CORS, Rate Limiting
- [x] TOON format & LLM optimization
- [x] WebSocket support
- [x] Template engine (Tera)
- [x] CLI tool (cargo-rustapi)
- [ ] **GraphQL support** (via async-graphql)
- [ ] **gRPC integration** (Tonic compatibility)
- [ ] **Distributed tracing** (OpenTelemetry)
- [ ] **Server-Sent Events** (SSE)
- [ ] **File upload/download** (multipart forms)
- [ ] **Caching layers** (Redis, in-memory)
- [ ] **Background jobs** (Tokio tasks, queues)
- [ ] **Health checks** (liveness/readiness probes)
- [ ] **Metrics** (Prometheus exporters)
- [ ] **HTTP/3 & QUIC** support

---

## � Use Cases & Who's Using RustAPI

### Perfect For:
- **🤖 AI/LLM APIs** — MCP servers, token-optimized responses
- **🚀 Startups** — Rapid prototyping with production-ready code
- **🏢 Microservices** — Service-to-service communication
- **📱 Mobile Backends** — Fast, type-safe REST APIs
- **🔄 Real-time Apps** — WebSocket support built-in
- **📊 Data Platforms** — High-performance data ingestion

### Projects Using RustAPI
> Building something with RustAPI? [Add your project](https://github.com/Tuntii/RustAPI/discussions)!

---

## �🆚 Comparison with Other Frameworks

| Feature | RustAPI | Axum | Actix-web | Rocket | FastAPI (Python) |
|---------|---------|------|-----------|--------|------------------|
| **Performance** | ⚡⚡⚡⚡⚡ | ⚡⚡⚡⚡ | ⚡⚡⚡⚡⚡ | ⚡⚡⚡ | ⚡ |
| **Learning Curve** | 📚📚 | 📚📚📚 | 📚📚📚📚 | 📚📚 | 📚 |
| **Auto OpenAPI** | ✅ Built-in | ⚠️ Manual | ⚠️ External | ✅ Limited | ✅ Built-in |
| **Validation** | ✅ Automatic | ⚠️ Manual | ⚠️ Manual | ✅ Basic | ✅ Pydantic |
| **JWT Auth** | ✅ Built-in | ⚠️ External | ⚠️ External | ❌ | ⚠️ External |
| **WebSocket** | ✅ Built-in | ✅ Built-in | ✅ Built-in | ❌ | ✅ Built-in |
| **LLM/TOON** | ✅ Unique | ❌ | ❌ | ❌ | ❌ |
| **Zero Config** | ✅ `auto()` | ⚠️ Manual | ⚠️ Manual | ⚠️ Manual | ✅ Auto |
| **Stability** | ✅ Facade | ⚠️ Direct | ⚠️ Direct | ⚠️ Direct | ✅ Stable |
| **Async/Await** | ✅ Native | ✅ Native | ✅ Native | ⚠️ Limited | ✅ Native |

### Why Choose RustAPI?

- **🎯 5-Line APIs** — Fastest time-to-production
- **🛡️ Facade Pattern** — Internal upgrades don't break your code
- **🤖 AI-Ready** — TOON format, MCP servers, LLM optimization
- **🎁 Batteries Included** — JWT, CORS, Rate Limiting, OpenAPI — all built-in
- **📚 Better DX** — FastAPI's ergonomics in Rust

---

## 🤝 Contributing

We welcome contributions! Here's how you can help:

1. **⭐ Star this repo** — Helps others discover RustAPI
2. **🐛 Report bugs** — Open an issue with reproduction steps
3. **💡 Suggest features** — Share your ideas in Discussions
4. **📝 Improve docs** — Fix typos, add examples
5. **🔧 Submit PRs** — See [CONTRIBUTING.md](CONTRIBUTING.md)

### Development Setup

```bash
# Clone the repo
git clone https://github.com/Tuntii/RustAPI.git
cd RustAPI

# Run tests
cargo test --all

# Run benchmarks
cd benches && ./run_benchmarks.ps1

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --all-targets --all-features
```

---

## 📞 Community & Support

- **📖 Documentation**: [docs.rs/rustapi-rs](https://docs.rs/rustapi-rs)
- **💬 Discussions**: [GitHub Discussions](https://github.com/Tuntii/RustAPI/discussions)
- **🐦 Twitter**: [@Tuntii](https://twitter.com/Tuntii)
- **🌐 Website**: [tunti35.com/projects/rustapi](https://www.tunti35.com/projects/rustapi)
- **📧 Email**: [tunahan@tunti35.com](mailto:tunahan@tunti35.com)

---

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Tuntii/RustAPI&type=Date)](https://star-history.com/#Tuntii/RustAPI&Date)

*If you find RustAPI useful, please consider giving it a star! It helps others discover the project.*

---

## License

MIT or Apache-2.0, at your option.
