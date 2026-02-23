<div align="center">
  <img src="https://raw.githubusercontent.com/Tuntii/RustAPI/refs/heads/main/assets/logo.jpg" alt="RustAPI" width="200" />

  # RustAPI

  **A high-performance, AI-native web framework for Rust.**

  [![Crates.io](https://img.shields.io/crates/v/rustapi-rs.svg)](https://crates.io/crates/rustapi-rs)
  [![Docs](https://img.shields.io/badge/docs-cookbook-brightgreen)](docs/cookbook/src/SUMMARY.md)
  [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
  [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Tuntii/RustAPI)
</div>

---

RustAPI is a Rust web framework built on **hyper 1.x** and **tokio** that combines FastAPI-style ergonomics with Rust's zero-cost abstractions. It provides a stable **facade architecture**, first-class OpenAPI generation, and an integrated AI runtime for building LLM-powered backends.

## Overview

- **Facade architecture** — User code imports only `rustapi-rs`; internal crates evolve independently without breaking the public API.
- **Three-tier request execution** — Ultra-fast path (no middleware), fast path (interceptors only), and full path (complete middleware stack) for optimal per-route overhead.
- **Auto-discovery routing** — Handlers decorated with `#[get]`/`#[post]`/… are collected at link time via `linkme` distributed slices. No manual route registration needed.
- **Native OpenAPI & Swagger UI** — Schema-first types generate specs at compile time; Swagger UI available at `/docs` by default.
- **AI-native runtime** — Built-in crates for LLM routing, agent orchestration, tool execution, and memory — making Rust a first-class language for AI backend development.

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
async fn main() {
    RustApi::auto().run("127.0.0.1:8080").await
}
```

`RustApi::auto()` discovers all annotated handlers, generates OpenAPI documentation, and starts a multi-threaded tokio runtime. The result:

- Swagger UI at `/docs`
- Input validation via `#[derive(Validate)]`
- Typed extractors: `Path`, `Query`, `Json`, `State`, `Headers`, `Cookies`, `AuthUser`

A shorter import prefix is also supported:

```toml
[dependencies]
api = { package = "rustapi-rs", version = "0.1.335" }
```

```rust
use api::prelude::*;

#[api::get("/users")]
async fn list_users() -> &'static str { "ok" }
```

## Architecture

```
User Code
    └── rustapi-rs (public facade)
            ├── rustapi-core       HTTP engine, router (radix tree), extractors, server
            ├── rustapi-macros     #[get], #[post], #[derive(Schema)], auto-discovery
            ├── rustapi-openapi    OpenAPI spec generation, Swagger UI
            ├── rustapi-validate   Input validation (validator crate integration)
            ├── rustapi-extras     JWT, CORS, rate-limit, replay, circuit-breaker, …
            ├── rustapi-toon       TOON format for token-efficient LLM responses
            ├── rustapi-ws         WebSocket (tokio-tungstenite, permessage-deflate)
            ├── rustapi-view       Tera template rendering
            ├── rustapi-grpc       gRPC via Tonic/Prost, side-by-side HTTP+gRPC
            ├── rustapi-jobs       Background job processing (Redis / Postgres)
            ├── rustapi-testing    TestClient for in-memory integration testing
            └── rustapi-ai         AI-native runtime (see below)
                    ├── rustapi-context    Request context, cost tracking, trace tree
                    ├── rustapi-memory     Pluggable memory (in-memory, Redis, vector DB)
                    ├── rustapi-tools      Tool registry and DAG execution graph
                    ├── rustapi-agent      Step-based agent engine with planning & replay
                    └── rustapi-llm        Model-agnostic LLM router (OpenAI, Anthropic, local)
```

All user-facing imports come from `rustapi-rs`. Internal crate boundaries are an implementation detail.

## AI-Native Runtime

RustAPI ships a dedicated AI runtime stack (`ai-*` feature flags) designed for building LLM-powered backends in Rust without external orchestration frameworks.

```
HTTP Request
    │
    ▼
RequestContext   ← cost budget, trace tree, event bus
    │
    ▼
AgentEngine      ← step-based execution loop
    ├── Planner        (decomposes goals into execution plans)
    ├── ToolGraph      (DAG of tool calls with dependency resolution)
    ├── MemoryStore    (conversation history + semantic memory)
    └── LlmRouter     (cost-aware routing with fallback chains)
    │
    ▼
StructuredOutput<T>  ← schema-first guaranteed decoding
    │
    ▼
HTTP Response    (Json / Toon / SSE stream)
```

### Key Components

| Crate | Responsibility |
|:------|:---------------|
| `rustapi-context` | Per-request context carrying auth, metadata, atomic cost counters, and a hierarchical trace tree |
| `rustapi-memory` | Trait-based memory abstraction with `InMemoryStore` (dev), Redis (production), and vector DB (semantic search) backends |
| `rustapi-tools` | `Tool` trait + `ToolRegistry` for runtime discovery; `ToolGraph` executes DAGs with parallel, sequential, and conditional nodes |
| `rustapi-agent` | `Step` trait and `AgentEngine` orchestrator with `Planner` strategies (static, ReAct), branching, and deterministic replay |
| `rustapi-llm` | `LlmProvider` abstraction over OpenAI, Anthropic, and local models; `LlmRouter` handles cost/latency/quality routing with circuit breakers |
| `rustapi-ai` | Unified facade: `AiRuntime` builder wires all components into a single `State`-injectable object |

### Usage

```rust
use rustapi_ai::prelude::*;

let runtime = AiRuntime::builder()
    .memory(InMemoryStore::new())
    .llm(LlmRouter::builder()
        .provider(MockProvider::new("dev"))
        .build())
    .build();

// Attach to RustAPI as shared state
RustApi::new()
    .state(runtime)
    .route("/chat", post(chat_handler))
    .run("0.0.0.0:8080").await;
```

Enable via feature flags in `Cargo.toml`:

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["ai-openai", "ai-redis"] }
```

## TOON: Token-Optimized Responses

Standard JSON is verbose when consumed by LLMs. The **TOON (Token-Oriented Object Notation)** format reduces response token counts by 50-58% — directly cutting API costs for token-billed providers (GPT-4o, Claude, etc.).

Enable with `features = ["protocol-toon"]` and use `Toon<T>` as a drop-in replacement for `Json<T>`.

## Time-Travel Debugging

The Replay middleware records HTTP request/response pairs and enables deterministic replay for debugging production issues.

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

Features: automatic recording, sensitive header redaction, in-memory or filesystem storage with TTL cleanup, and `ReplayClient` for programmatic test automation.

## Feature Flags

Features are organized into four namespaces:

| Namespace | Purpose | Examples |
|:----------|:--------|:--------|
| `core-*` | Core framework capabilities | `core-openapi` (default), `core-tracing` (default), `core-http3`, `core-compression` |
| `protocol-*` | Optional protocol support | `protocol-toon`, `protocol-ws`, `protocol-view`, `protocol-grpc` |
| `extras-*` | Production middleware | `extras-jwt`, `extras-cors`, `extras-rate-limit`, `extras-replay`, `extras-otel` |
| `ai-*` | AI-native runtime | `ai-core`, `ai-openai`, `ai-anthropic`, `ai-local`, `ai-redis` |

Aggregate flags: `core` (default), `protocol-all`, `extras-all`, `ai-full`, `full` (everything).

## Comparison

| | **RustAPI** | Actix-web | Axum | FastAPI (Python) |
|:---|:---:|:---:|:---:|:---:|
| Throughput (hello-world) | ~92k req/s | ~105k | ~100k | ~12k |
| Ergonomics | High | Low | Medium | High |
| Auto OpenAPI | Built-in | Manual | Manual | Built-in |
| AI/LLM Runtime | Integrated | — | — | — |
| Request Replay | Built-in | — | — | 3rd-party |
| API Stability | Facade | Direct | Direct | Stable |

Benchmarks run on the same hardware with default configurations. See [performance docs](docs/cookbook/src/concepts/performance.md) for methodology.

## v0.1.335 Highlights

- **Dual-stack runtime** — Simultaneous HTTP/1.1 (TCP) and HTTP/3 (QUIC/UDP)
- **WebSocket compression** — Full permessage-deflate negotiation
- **AI-native runtime** — `rustapi-ai` facade with LLM routing, agent engine, tool graphs, and pluggable memory
- **gRPC support** — Optional `rustapi-grpc` crate with Tonic/Prost integration and side-by-side `run_rustapi_and_grpc` runner
- **Async validation** — Deep integration with application state for context-dependent rules
- **CLI improvements** — `cargo rustapi new` interactive scaffolding with gRPC and AI feature selection

## Roadmap

- [x] gRPC integration via Tonic (`rustapi-grpc`)
- [x] Distributed tracing with OpenTelemetry (`extras-otel`)
- [x] AI-native runtime (context, memory, tools, agents, LLM routing)
- [ ] RustAPI Cloud — managed deployment to major cloud providers

## Documentation

Detailed architecture guides, recipes, and deep-dives live in the **[Cookbook](docs/cookbook/src/SUMMARY.md)**.

- [System Architecture](docs/cookbook/src/architecture/system_overview.md)
- [Performance Benchmarks](docs/cookbook/src/concepts/performance.md)
- [gRPC Integration](docs/cookbook/src/crates/rustapi_grpc.md)
- [Examples](crates/rustapi-rs/examples/)

## Development

```sh
cargo build --workspace --all-features   # Build everything
cargo test --workspace                   # Run tests
cargo clippy --workspace -- -D warnings  # Lint
./scripts/check_quality.ps1              # Full quality gate (check + clippy + test)
cargo run -p rustapi-rs --example pins_api  # Run an example
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

---

<div align="center">
  <sub>Built by Tunti35.</sub>
</div>
