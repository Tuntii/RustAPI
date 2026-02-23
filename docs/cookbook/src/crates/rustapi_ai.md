# rustapi-ai: The Brain

The `rustapi-ai` crate is the **unified facade** for RustAPI's AI-Native Backend Runtime. It re-exports and wires together six specialized crates into a single cohesive `AiRuntime`:

```
rustapi-ai (facade)
├── rustapi-context → RequestContext, CostBudget, EventBus
├── rustapi-memory  → InMemoryStore, RedisStore, ConversationMemory
├── rustapi-tools   → ToolRegistry, ToolGraph, ClosureTool
├── rustapi-agent   → AgentEngine, Planner, ReActPlanner, ReplayEngine
└── rustapi-llm     → LlmRouter, providers (OpenAI, Anthropic)
```

## Crate Hierarchy

Users should import from the facade:

```rust,ignore
// ✅ Correct — use the facade
use rustapi_rs::ai::prelude::*;

// ❌ Avoid — internal crate paths
use rustapi_context::RequestContext;
```

## AiRuntime

The central builder that wires everything together:

```rust,no_run
use rustapi_rs::ai::prelude::*;

let runtime = AiRuntime::builder()
    .memory(InMemoryStore::new())
    // .llm_router(router)     — optional, add when LLM is needed
    // .tool_registry(tools)   — optional, add when tools are needed
    .build();
```

`AiRuntime` is `Clone + Send + Sync` and designed to be shared via `State<AiRuntime>` or the `AiRt` extractor.

## Feature Flags

| Feature | What it enables |
|---------|----------------|
| `ai-core` | Base runtime, in-memory store, tools, agents |
| `ai-openai` | OpenAI Chat Completions provider |
| `ai-anthropic` | Anthropic Messages API provider |
| `ai-redis` | Redis-backed memory store |
| `ai-http` | `AiContextLayer` middleware + `AiCtx`/`AiRt` extractors |
| `ai-full` | Everything above |

## Sub-Crate Overview

### rustapi-context
Per-request lifecycle management:
- **`RequestContext`** — method, path, trace IDs, metadata, auth info
- **`CostBudget`** — per_request_tokens / per_request_usd / unlimited
- **`SharedCostTracker`** — `record_tokens()`, `record_cost_usd()`, `snapshot()`
- **`EventBus`** — pub/sub execution events (tool_started, llm_completed, etc.)
- **`TraceTree`** — hierarchical execution tracing

### rustapi-memory
Pluggable key-value storage with conversation tracking:
- **`MemoryStore`** trait — store/get/list/delete/count/clear
- **`InMemoryStore`** — HashMap-based, TTL support, capacity limits
- **`RedisStore`** (feature: `redis`) — Production Redis backend with EXPIRE
- **`ConversationMemory`** — Multi-turn conversation history with roles

### rustapi-tools
Function calling and DAG execution:
- **`Tool`** trait — name, description, parameters, async execute
- **`ClosureTool`** — Quick inline tool definitions
- **`ToolRegistry`** — Register, list, and call tools by name
- **`ToolGraph`** — Directed acyclic graph for multi-tool orchestration

### rustapi-agent
AI agent planning and execution:
- **`Planner`** trait — Generate execution plans from context
- **`StaticPlanner`** — Fixed tool sequence
- **`ReActPlanner`** — Reason-Act-Observe loop
- **`AgentEngine`** — Execute plans with tool calling
- **`ReplayEngine`** — Record and replay for deterministic testing

### rustapi-llm
Provider-agnostic LLM routing:
- **`LlmRouter`** — Route requests to models with fallback
- **`LlmProvider`** trait — complete, complete_stream, embeddings
- **`OpenAiProvider`** (feature: `openai`) — OpenAI Chat Completions API
- **`AnthropicProvider`** (feature: `anthropic`) — Anthropic Messages API

## HTTP Middleware (feature: `ai-http`)

Three components bridge the AI runtime with HTTP handlers:

| Component | Purpose |
|-----------|---------|
| `AiContextLayer` | Middleware that auto-creates `RequestContext` per request |
| `AiCtx` | Extractor for `RequestContext` |
| `AiRt` | Extractor for `AiRuntime` |

```rust,ignore
let app = RustApi::new()
    .layer(AiContextLayer::new(runtime.clone()))
    .route("/chat", post(handler));

async fn handler(AiCtx(ctx): AiCtx, AiRt(rt): AiRt) -> Result<Json<Value>> {
    // ctx: per-request context with cost tracking
    // rt: shared AI runtime with memory, tools, LLM
}
```

## Testing

All AI crates use `rustapi-testing::TestClient` for in-memory integration tests. The test suite runs without any external services (Redis, LLM APIs) — only unit-testable components are exercised by default.

```bash
# Run all AI tests
cargo test -p rustapi-ai
cargo test -p rustapi-context
cargo test -p rustapi-memory
cargo test -p rustapi-tools
cargo test -p rustapi-agent
cargo test -p rustapi-llm

# With provider features
cargo test -p rustapi-llm --features openai,anthropic

# With Redis
cargo test -p rustapi-memory --features redis
```
