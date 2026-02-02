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

Top-tier LLMs (Claude, GPT-4) charge by the token. RustAPI's TOON format reduces response token counts by **50-58%** compared to standard JSON.

*   **💰 Save 50% on API Costs**: Half the tokens, same data.
*   **🌊 Zero-Latency Streaming**: Built for real-time AI agents.
*   **🔌 MCP-Ready**: Out-of-the-box support for Model Context Protocol.

> "RustAPI isn't just a web server; it's the native language of your AI agents."

## 🥊 Dare to Compare

We optimize for **Developer Joy** without sacrificing **Req/Sec**.

| Feature | **RustAPI** | Actix-web | Axum | FastAPI (Python) |
|:-------|:-----------:|:---------:|:----:|:----------------:|
| **Performance** | **~92k req/s** | ~105k | ~100k | ~12k |
| **DX (Simplicity)** | 🟢 **High** | 🔴 Low | 🟡 Medium | 🟢 High |
| **Boilerplate** | **Zero** | High | Medium | Zero |
| **AI/LLM Native** | ✅ **Yes** | ❌ No | ❌ No | ❌ No |
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

**That's it.** You get:
*   ✅ **Swagger UI** at `/docs`
*   ✅ **Input Validation**
*   ✅ **Multi-threaded Runtime**
*   ✅ **Zero Config**

## 🗺️ Public Roadmap: Next 30 Days

We build in public. Here is our immediate focus for **February 2026**:

*   [ ] **Visual Status Page**: Automatic health dashboard for all endpoints.
*   [ ] **gRPC Integration**: First-class support via Tonic.
*   [ ] **Distributed Tracing**: One-line OpenTelemetry setup.
*   [ ] **RustAPI Cloud**: One-click deploy to major cloud providers.

## 📚 Documentation

We moved our detailed architecture, recipes, and deep-dives to the **[Cookbook](docs/cookbook/src/SUMMARY.md)**.

*   [System Architecture & Diagrams](docs/cookbook/src/architecture/system_overview.md)
*   [Performance Benchmarks](docs/cookbook/src/concepts/performance.md)
*   [Full Examples](crates/rustapi-rs/examples/)

---

<div align="center">
  <sub>Built with ❤️ by the Tunti3.</sub>
</div>
