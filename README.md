<div align="center">
  <img src="https://raw.githubusercontent.com/Tuntii/RustAPI/refs/heads/main/assets/logo.jpg" alt="RustAPI" width="200" />
  
  # RustAPI
  
  **Rust Speed. Python Simplicity. AI Efficiency.**

  [![Crates.io](https://img.shields.io/crates/v/rustapi-rs.svg)](https://crates.io/crates/rustapi-rs)
  [![Docs](https://img.shields.io/badge/docs-cookbook-brightgreen)](docs/cookbook/src/SUMMARY.md)
  [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
  [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Tuntii/RustAPI)
</div>

---

## ⚡ Why RustAPI?

**Most Rust frameworks force you to choose: Speed (Actix) OR Ergonomics (Axum).**
RustAPI gives you **both**.

We built the framework we wanted: **FastAPI's developer experience** backed by **Rust's raw performance**. 
No boilerplate. No fighting the borrow checker for simple handlers. Just code that flies.

## 🧠 The Killer Feature: AI-First Architecture

**Problem:** Standard JSON APIs are verbose and expensive for Large Language Models (LLMs).
**Solution:** RustAPI natively supports **TOON (Token-Oriented Object Notation)**.

Top-tier LLMs (Claude, GPT-4o) charge by the token. RustAPI's TOON format reduces response token counts by **50-58%** compared to standard JSON.

*   **💰 Save 50% on API Costs**: Half the tokens, same data.
*   **🌊 Zero-Latency Streaming**: Built for real-time AI agents.
*   **🔌 MCP-Ready**: Out-of-the-box support for Model Context Protocol.

> "RustAPI isn't just a web server; it's the native language of your AI agents."

## 🔄 Time-Travel Debugging (NEW in v0.1.300)

**Production debugging shouldn't be a nightmare.** RustAPI's Replay system records and replays HTTP requests with surgical precision.

```rust
// 1. Enable replay recording in production
RustApi::new()
    .layer(ReplayLayer::new(store, config))
    .run("0.0.0.0:8080").await;

// 2. Replay ANY request from the CLI
$ cargo rustapi replay list
$ cargo rustapi replay run <id> --target http://localhost:8080
$ cargo rustapi replay diff <id> --target http://staging
```

**What makes it special:**
*   🎬 **Zero-Code Recording**: Middleware automatically captures request/response pairs
*   🔐 **Security First**: Sensitive headers redacted, bearer auth required, disabled by default
*   💾 **Flexible Storage**: In-memory (dev) or filesystem (production) with TTL cleanup
*   🧪 **Integration Testing**: `ReplayClient` for programmatic test automation
*   🕵️ **Root Cause Analysis**: Replay exact production failures in local environment

> "Fix production bugs in 5 minutes instead of 5 hours."

## 🥊 Dare to Compare

We optimize for **Developer Joy** without sacrificing **Req/Sec**.

| Feature | **RustAPI** | Actix-web | Axum | FastAPI (Python) |
|:-------|:-----------:|:---------:|:----:|:----------------:|
| **Performance** | **~92k req/s** | ~105k | ~100k | ~12k |
| **DX (Simplicity)** | 🟢 **High** | 🔴 Low | 🟡 Medium | 🟢 High |
| **Boilerplate** | **Zero** | High | Medium | Zero |
| **AI/LLM Native** | ✅ **Yes** | ❌ No | ❌ No | ❌ No |
| **Time-Travel Debug** | ✅ **Built-in** | ❌ No | ❌ No | ⚠️ 3rd-party |
| **Stability Logic** | 🛡️ **Facade** | ⚠️ Direct | ⚠️ Direct | ✅ Stable |

## 🚀 30-Second Start

Write your API in 5 lines. It's that simple.

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize, Schema)]
struct Message { text: String }

#[rustapi_rs::get("/hello/{name}")]
async fn hello(Path(name): Path<String>) -> Json<Message> {
    Json(Message { text: format!("Hello, {}!", name) })
}

#[rustapi_rs::main]
async fn main() {
    // 1 line to rule them all: Auto-discovery, OpenAPI, Validation
    RustApi::auto().run("127.0.0.1:8080").await
}
```

Prefer a shorter macro prefix? You can rename the crate in `Cargo.toml` and use the same macros:

```toml
[dependencies]
api = { package = "rustapi-rs", version = "0.1.335" }
```

```rust
use api::prelude::*;

#[api::get("/users")]
async fn list_users() -> &'static str { "ok" }
```

**That's it.** You get:
*   ✅ **Swagger UI** at `/docs`
*   ✅ **Input Validation**
*   ✅ **Multi-threaded Runtime**
*   ✅ **Zero Config**

## Feature Taxonomy (Stable)

RustAPI now groups features into three namespaces:

| Namespace | Purpose | Examples |
|:--|:--|:--|
| `core-*` | Core framework capabilities | `core-openapi`, `core-tracing`, `core-http3` |
| `protocol-*` | Optional protocol crates | `protocol-toon`, `protocol-ws`, `protocol-view`, `protocol-grpc` |
| `extras-*` | Optional production middleware/integrations | `extras-jwt`, `extras-cors`, `extras-rate-limit`, `extras-replay` |

Meta features:
- `core` (default)
- `protocol-all`
- `extras-all`
- `full = core + protocol-all + extras-all`

## ✨ Latest Release Highlights (v0.1.335)

*   ✅ **Dual-Stack Runtime**: Simultaneous HTTP/1.1 (TCP) and HTTP/3 (QUIC/UDP) support
*   ✅ **WebSocket**: Full permessage-deflate negotiation and compression
*   ✅ **OpenAPI**: Improved reference integrity and native validation docs
*   ✅ **Async Validation**: Deep integration with application state for complex rules
*   ✅ **gRPC Foundation**: New optional `rustapi-grpc` crate with Tonic/Prost integration and side-by-side HTTP + gRPC runners (`run_rustapi_and_grpc`, `run_rustapi_and_grpc_with_shutdown`)
*   ✅ **CLI DX Update**: `cargo rustapi new` interactive feature selection now includes `grpc`

## 🗺️ Public Roadmap: Next 30 Days

We build in public. Here is our immediate focus for **February 2026**:

*   [x] **Visual Status Page**: Automatic health dashboard for all endpoints.
*   [x] **gRPC Integration (Foundation)**: First-class optional crate via Tonic (`rustapi-grpc`) with RustAPI facade-level feature flag support.
*   [x] **Distributed Tracing**: One-line OpenTelemetry setup.
*   [ ] **RustAPI Cloud**: One-click deploy to major cloud providers.

## 📚 Documentation

We moved our detailed architecture, recipes, and deep-dives to the **[Cookbook](docs/cookbook/src/SUMMARY.md)**.

*   [System Architecture & Diagrams](docs/cookbook/src/architecture/system_overview.md)
*   [Performance Benchmarks](docs/cookbook/src/concepts/performance.md)
*   [gRPC Integration Guide](docs/cookbook/src/crates/rustapi_grpc.md)
*   [Full Examples](crates/rustapi-rs/examples/)

---

<div align="center">
  <sub>Built with ❤️ by the Tunti3.</sub>
</div>
