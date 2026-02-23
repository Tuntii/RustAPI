# AI-Native Runtime

RustAPI ships a full **AI-Native Backend Runtime** that lets you build intelligent APIs with first-class support for LLM orchestration, tool execution, memory persistence, and agent pipelines — all within the same Rust process that serves your HTTP endpoints.

```
HTTP Request → Context → Agent Engine → Tool Graph → Memory → LLM → Structured Response
```

## Feature Flags

Enable AI features in your `Cargo.toml`:

```toml
[dependencies]
# Minimal — context, memory, tools, agents, LLM routing (in-memory)
rustapi-rs = { version = "0.1", features = ["ai-core"] }

# With OpenAI provider
rustapi-rs = { version = "0.1", features = ["ai-core", "ai-openai"] }

# With Anthropic provider
rustapi-rs = { version = "0.1", features = ["ai-core", "ai-anthropic"] }

# Redis-backed memory
rustapi-rs = { version = "0.1", features = ["ai-core", "ai-redis"] }

# HTTP middleware (auto-creates AI context per request)
rustapi-rs = { version = "0.1", features = ["ai-core", "ai-http"] }

# Everything
rustapi-rs = { version = "0.1", features = ["ai-full"] }
```

## Architecture

The AI runtime is composed of six crates, all re-exported through a single `rustapi_rs::ai` module:

| Crate | Purpose |
|-------|---------|
| `rustapi-context` | Per-request context, cost tracking, event bus, observability |
| `rustapi-memory` | Pluggable storage: in-memory, Redis, conversation history |
| `rustapi-tools` | Tool registry, function calling, directed acyclic tool graphs |
| `rustapi-agent` | Planning engines (ReAct, static), step execution, replay |
| `rustapi-llm` | Provider-agnostic LLM routing (OpenAI, Anthropic, local) |
| `rustapi-ai` | Unified `AiRuntime` builder, HTTP middleware, extractors |

---

## Quick Start

```rust,no_run
use rustapi_rs::ai::prelude::*;

#[tokio::main]
async fn main() {
    // 1. Build the AI runtime
    let runtime = AiRuntime::builder()
        .memory(InMemoryStore::new())
        .build();

    // 2. Create a per-request context
    let ctx = RequestContextBuilder::new("chat", "/api/chat")
        .with_cost_budget(CostBudget::per_request_tokens(4000))
        .build();

    // 3. Register tools
    let mut tools = ToolRegistry::new();
    tools.register(ClosureTool::new(
        "get_weather",
        "Get current weather for a city",
        vec![("city", "string", "City name", true)],
        |args| Box::pin(async move {
            let city = args["city"].as_str().unwrap_or("unknown");
            Ok(ToolOutput::text(format!("22°C and sunny in {city}")))
        }),
    ));

    // 4. Run an agent plan
    let planner = StaticPlanner::new(vec!["get_weather"]);
    let engine = AgentEngine::new(planner, tools);
    let result = engine
        .execute(
            AgentContext::new(ctx.clone())
                .with_input(serde_json::json!({"city": "Istanbul"})),
        )
        .await;

    println!("Result: {:?}", result);
}
```

---

## Request Context

Every AI operation starts with a `RequestContext` — a structured envelope carrying the request method, path, trace IDs, cost budgets, authentication info, and an event bus.

```rust,no_run
use rustapi_rs::ai::prelude::*;

let ctx = RequestContextBuilder::new("POST", "/api/chat")
    .with_cost_budget(CostBudget::per_request_tokens(8000))
    .with_auth(AuthContext::bearer("user-42", vec!["admin"]))
    .with_metadata("session_id", serde_json::json!("abc-123"))
    .build();

// Access observability info
println!("Request ID: {}", ctx.id());
println!("Trace ID: {}", ctx.observability().trace_id);

// Track costs
ctx.cost_tracker().record_tokens(150);
let snapshot = ctx.cost_tracker().snapshot();
println!("Tokens used: {}", snapshot.total_tokens);
```

### Event Bus

Subscribe to execution events for logging, metrics, or streaming:

```rust,no_run
use rustapi_rs::ai::prelude::*;

let ctx = RequestContextBuilder::new("GET", "/api/status").build();
let mut subscriber = ctx.event_bus().subscribe();

// In another task, listen for events
tokio::spawn(async move {
    while let Some(event) = subscriber.recv().await {
        println!("Event: {:?}", event);
    }
});

// Emit events during processing
ctx.event_bus().emit(ExecutionEvent::tool_started("get_weather"));
```

---

## Memory

### In-Memory Store (Default)

```rust,no_run
use rustapi_rs::ai::prelude::*;

let store = InMemoryStore::new();

// Store an entry
let entry = MemoryEntry::new("user:42:prefs", serde_json::json!({"theme": "dark"}))
    .with_namespace("session-abc")
    .with_ttl(3600)
    .with_metadata("type", serde_json::json!("preferences"));
store.store(entry).await.unwrap();

// Query entries
let results = store.list(
    &MemoryQuery::new()
        .with_namespace("session-abc")
        .with_key_prefix("user:42")
        .with_limit(10),
).await.unwrap();
```

### Redis Store

Enable with `ai-redis` feature. Entries are serialized as JSON in Redis with native `EXPIRE` for TTL:

```rust,ignore
use rustapi_rs::ai::prelude::*;

// Connect to Redis
let store = RedisStore::new("redis://127.0.0.1:6379", "myapp").await?;

// Same MemoryStore API as InMemoryStore
store.store(
    MemoryEntry::new("key", serde_json::json!("value"))
        .with_namespace("production")
        .with_ttl(300),
).await?;

let entry = store.get("key").await?;
```

### Conversation Memory

Track multi-turn conversations with role-based turns:

```rust,no_run
use rustapi_rs::ai::prelude::*;

let store = InMemoryStore::new();
let conv = ConversationMemory::new(Box::new(store.clone()));

// Record turns
conv.add_turn("session-1", Turn::user("What's the weather?")).await.unwrap();
conv.add_turn("session-1", Turn::assistant("It's 22°C in Istanbul.")).await.unwrap();

// Retrieve recent context
let history = conv.get_recent_turns("session-1", 10).await.unwrap();
assert_eq!(history.len(), 2);
```

---

## Tool System

### Closure Tools

Define tools inline with `ClosureTool`:

```rust,no_run
use rustapi_rs::ai::prelude::*;

let tool = ClosureTool::new(
    "calculate",
    "Evaluate a math expression",
    vec![("expr", "string", "Math expression", true)],
    |args| Box::pin(async move {
        let expr = args["expr"].as_str().unwrap_or("0");
        Ok(ToolOutput::text(format!("Result: {expr}")))
    }),
);
```

### Tool Registry

```rust,no_run
use rustapi_rs::ai::prelude::*;

let mut registry = ToolRegistry::new();
registry.register(ClosureTool::new(
    "search", "Search the database",
    vec![("q", "string", "Query", true)],
    |args| Box::pin(async move {
        Ok(ToolOutput::json(serde_json::json!({"results": []})))
    }),
));

// List available tools
let descriptions = registry.list_tools();

// Execute by name
let output = registry
    .call("search", serde_json::json!({"q": "rust"}))
    .await
    .unwrap();
```

### Tool Graph (DAG Execution)

Execute multiple tools as a directed acyclic graph with conditional branching:

```rust,no_run
use rustapi_rs::ai::prelude::*;

let mut registry = ToolRegistry::new();
// ... register tools ...

let graph = ToolGraph::builder()
    .node(ToolNode::tool("fetch", "search_db"))
    .node(
        ToolNode::tool("transform", "format_results")
            .depends_on("fetch"),
    )
    .build()
    .unwrap();

let output = graph
    .execute(&registry, serde_json::json!({"query": "rust"}))
    .await
    .unwrap();
```

---

## Agent Engine

### Static Planner

Execute a fixed sequence of tools:

```rust,no_run
use rustapi_rs::ai::prelude::*;

let planner = StaticPlanner::new(vec!["fetch_data", "transform", "respond"]);
let engine = AgentEngine::new(planner, registry);

let result = engine
    .execute(AgentContext::new(ctx).with_input(serde_json::json!({"q": "hello"})))
    .await;
```

### ReAct Planner

Reason-Act-Observe loop for dynamic tool selection:

```rust,no_run
use rustapi_rs::ai::prelude::*;

let planner = ReActPlanner::new()
    .max_iterations(5)
    .stop_condition(|output| output.contains("FINAL ANSWER"));

let engine = AgentEngine::new(planner, registry);
let result = engine.execute(agent_ctx).await;
```

### Replay & Debugging

Record and replay agent executions for deterministic testing:

```rust,no_run
use rustapi_rs::ai::prelude::*;

// Record a session
let replay_engine = ReplayEngine::new();
let session = replay_engine.record(engine, agent_ctx).await;

// Replay deterministically
let replayed = replay_engine.replay(&session).await;
assert_eq!(session.result, replayed.result);
```

---

## LLM Routing

The `LlmRouter` provides provider-agnostic request routing with fallback chains:

```rust,no_run
use rustapi_rs::ai::prelude::*;

let router = LlmRouter::builder()
    .default_model("gpt-4o")
    .add_model(ModelConfig::new("gpt-4o").with_provider("openai"))
    .add_model(ModelConfig::new("claude-3-5-sonnet").with_provider("anthropic"))
    .build();

// Route a completion request
let response = router
    .complete(LlmRequest::new("Explain Rust ownership in one sentence"))
    .await?;

println!("{}", response.content);
println!("Tokens: {}", response.usage.total_tokens);
```

### OpenAI Provider

```rust,ignore
use rustapi_rs::ai::llm::providers::openai::{OpenAiProvider, OpenAiConfig};

let provider = OpenAiConfig::builder()
    .api_key("sk-...")
    .default_model("gpt-4o")
    .build()
    .unwrap();

let response = provider
    .complete(&LlmRequest::new("Hello!"))
    .await?;
```

### Anthropic Provider

```rust,ignore
use rustapi_rs::ai::llm::providers::anthropic::{AnthropicProvider, AnthropicConfig};

let provider = AnthropicConfig::builder()
    .api_key("sk-ant-...")
    .default_model("claude-3-5-sonnet-20241022")
    .build()
    .unwrap();

let response = provider
    .complete(&LlmRequest::new("Hello!"))
    .await?;
```

---

## HTTP Integration

Enable `ai-http` to get automatic AI context injection for every HTTP request:

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_rs::ai::prelude::*;

// Build the AI runtime
let runtime = AiRuntime::builder()
    .memory(InMemoryStore::new())
    .build();

// Add the AI middleware layer
let app = RustApi::new()
    .state(runtime.clone())
    .layer(AiContextLayer::new(runtime))
    .route("/chat", post(chat_handler));

// Handler with AI extractors
async fn chat_handler(
    AiCtx(ctx): AiCtx,     // Auto-injected RequestContext
    AiRt(rt): AiRt,        // Auto-injected AiRuntime
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    // Use ctx for cost tracking, tracing, event bus
    ctx.cost_tracker().record_tokens(100);

    // Use rt for memory, tools, agents
    Ok(Json(serde_json::json!({"status": "ok"})))
}
```

### Extractors

| Extractor | Type | Description |
|-----------|------|-------------|
| `AiCtx` | `RequestContext` | Per-request context with cost tracking, event bus |
| `AiRt` | `AiRuntime` | The full AI runtime (memory, tools, agents, LLM) |

Both implement `FromRequestParts`, so they can appear in any order and don't consume the request body.

---

## Cost Tracking

Built-in cost budgets prevent runaway LLM spending:

```rust,no_run
use rustapi_rs::ai::prelude::*;

let ctx = RequestContextBuilder::new("POST", "/chat")
    .with_cost_budget(CostBudget::per_request_tokens(4000))
    .build();

// Record usage
ctx.cost_tracker().record_tokens(1500);
ctx.cost_tracker().record_cost_usd(0.03);

// Check budget
let snapshot = ctx.cost_tracker().snapshot();
println!("Tokens: {}/{:?}", snapshot.total_tokens, snapshot.budget_tokens);
println!("Cost: ${:.4}/{:?}", snapshot.total_cost_usd, snapshot.budget_usd);
```

---

## Full Example: AI Chat API

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_rs::ai::prelude::*;

#[rustapi_rs::post("/api/chat")]
async fn chat(
    AiCtx(ctx): AiCtx,
    AiRt(rt): AiRt,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let session_id = body["session_id"].as_str().unwrap_or("default");
    let message = body["message"].as_str().unwrap_or("");

    // 1. Store the user message in conversation memory
    let conv = ConversationMemory::new(rt.memory_store());
    conv.add_turn(session_id, Turn::user(message)).await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // 2. Get conversation history for context
    let history = conv.get_recent_turns(session_id, 20).await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // 3. Build LLM request with context
    let prompt = history.iter()
        .map(|t| format!("{}: {}", t.role, t.content))
        .collect::<Vec<_>>()
        .join("\n");

    // 4. Route to LLM
    let response = rt.llm_router()
        .complete(LlmRequest::new(&prompt))
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // 5. Track cost
    ctx.cost_tracker().record_tokens(response.usage.total_tokens as u64);

    // 6. Store assistant response
    conv.add_turn(session_id, Turn::assistant(&response.content)).await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "response": response.content,
        "tokens_used": response.usage.total_tokens,
        "session_id": session_id,
    })))
}
```

---

## Testing AI Handlers

Use `TestClient` (from `rustapi-testing`) with the AI middleware:

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_rs::ai::prelude::*;
use rustapi_testing::TestClient;

#[tokio::test]
async fn test_chat_endpoint() {
    let runtime = AiRuntime::builder()
        .memory(InMemoryStore::new())
        .build();

    let app = RustApi::new()
        .layer(AiContextLayer::new(runtime.clone()))
        .route("/chat", post(chat));

    let client = TestClient::new(app);
    let response = client
        .post("/chat")
        .json(&serde_json::json!({
            "session_id": "test",
            "message": "Hello!"
        }))
        .send()
        .await;

    response.assert_status(200);
}
```
