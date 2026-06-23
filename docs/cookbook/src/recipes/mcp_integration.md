# MCP Integration (Agent Tools)

RustAPI can expose selected HTTP endpoints as **discoverable tools** for LLMs and AI agents (Claude, Cursor, custom multi-agent systems) using the native Model Context Protocol (MCP) support in the `rustapi-mcp` crate.

This gives you:
- Zero duplication: tool definitions come from your existing routes, `#[derive(Schema)]` types, and OpenAPI metadata.
- Safety by default: nothing is exposed unless you explicitly allow it via tags or path prefixes.
- Full pipeline respect: every `tools/call` is proxied (or executed in-process) through your normal RustAPI layers, extractors, validators, middleware, and error handling.
- Optional zero-overhead in-process dispatch (see dedicated recipe).
- `cargo rustapi mcp generate` turns any OpenAPI spec into an MCP server.
- stdio transport support for desktop AI clients.

## The Problem

You want an AI agent to call your business logic ("get the weather", "create an order", "run a report") without writing a separate agent interface, duplicating validation, or opening unsafe endpoints.

## The Solution

Tag the routes you are happy to expose to agents, then attach an `McpServer` that speaks the MCP protocol over HTTP+SSE/JSON-RPC. Run it side-by-side with your normal HTTP API using the provided concurrent runner (modeled after gRPC integration).

## Dependencies

```toml
[dependencies]
rustapi-rs = { version = "0.1.507", features = ["protocol-mcp"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1.0", features = ["derive"] }
```

## Implementation

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_rs::protocol::mcp::{InvocationMode, McpConfig, McpServer, run_rustapi_and_mcp_with_shutdown, ToolPolicy};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Schema)]
struct Weather {
    city: String,
    temperature: i32,
    unit: &'static str,
}

#[derive(Deserialize, Schema)]
struct ComputeRequest {
    a: i32,
    b: i32,
}

#[derive(Serialize, Schema)]
struct ComputeResponse {
    sum: i32,
}

// Only routes with the "agent" tag will be visible to MCP clients.
#[rustapi_rs::get("/weather/{city}")]
#[rustapi_rs::tag("agent")]
#[rustapi_rs::summary("Get the current weather for a city")]
async fn get_weather(Path(city): Path<String>) -> Json<Weather> {
    Json(Weather {
        city,
        temperature: 22,
        unit: "C",
    })
}

#[rustapi_rs::post("/compute")]
#[rustapi_rs::tag("agent")]
#[rustapi_rs::summary("Add two numbers (demo tool)")]
async fn compute(Json(req): Json<ComputeRequest>) -> Json<ComputeResponse> {
    Json(ComputeResponse { sum: req.a + req.b })
}

#[rustapi_rs::get("/admin/internal")]
async fn internal_only() -> &'static str {
    "you should never see this via MCP"
}

// MCP permission annotations (framework-level scoping)
#[rustapi_rs::post("/orders")]
#[rustapi_rs::tag("agent")]
#[rustapi_rs::mcp(write, require = "confirm")]   // agent must confirm
async fn create_order(Json(_body): Json<serde_json::Value>) -> &'static str {
    "order created"
}

#[rustapi_rs::get("/admin/secrets")]
#[rustapi_rs::tag("agent")]
#[rustapi_rs::mcp(skip)]                         // never expose to agents
async fn admin_secrets() -> &'static str {
    "top secret"
}

#[rustapi_rs::post("/webhooks/stripe")]
#[rustapi_rs::mcp(readonly)]                     // POST but safe (external callback)
async fn stripe_webhook() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = RustApi::auto();

    let mcp = McpServer::from_rustapi(
        &app,
        McpConfig::new()
            .name("my-awesome-api")
            .version("1.0.0")
            .description(Some("Business capabilities exposed to agents".into()))
            .allowed_tags(["agent"]) // <-- explicit and safe
            .invocation_mode(InvocationMode::InProcess), // zero-overhead direct calls
    );

    println!("HTTP API : http://127.0.0.1:8080");
    println!("MCP (agents) : http://127.0.0.1:9090");

    // Permission scoping example (recommended for agents):
    // Use tool_policy + route-level #[mcp(...)] attributes
    let mcp_safe = McpServer::from_rustapi(
        &app,
        McpConfig::new()
            .allowed_tags(["agent"])
            .tool_policy(ToolPolicy::ReadOnly) // only GETs etc. by default
            .invocation_mode(InvocationMode::InProcess),
    );

    // Run both servers. Tool calls use in-process dispatch for max speed
    // while still going through the full middleware stack.
    run_rustapi_and_mcp_with_shutdown(
        app,
        "0.0.0.0:8080",
        mcp,
        "0.0.0.0:9090",
        tokio::signal::ctrl_c(),
    )
    .await?;

    Ok(())
}
```

## How Tool Discovery Works

- `McpServer::from_rustapi` reads the OpenAPI spec that RustAPI already generates from your handlers and `Schema` types.
- Only operations whose tags intersect with `allowed_tags` (or that match `allowed_path_prefixes`) become MCP tools.
- Tool `name`, `description`, and `inputSchema` are derived automatically (operationId preferred, otherwise method+path slug).
- The internal `/admin/internal` route above will **never** appear because it has no matching tag.

## Calling Tools from an Agent

MCP clients speak a small JSON-RPC dialect. Example raw interaction (you normally don't write this by hand):

```json
// initialize
{ "jsonrpc": "2.0", "id": 1, "method": "initialize" }

// list available tools (only the tagged ones)
{ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }

// call a tool
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "get_weather",
    "arguments": { "city": "Istanbul" }
  }
}
```

The result comes back wrapped in the standard MCP `content` array with an `isError` flag.

## Security & Operational Notes

- **Explicit exposure is the default.** An empty `allowed_tags` + no prefixes means zero tools are advertised.
- Every tool invocation goes through your normal `RustApi` stack (rate limiting, auth layers, body limits, validation, audit, replay recording, etc.).
- Use `admin_token` in `McpConfig` if you want an extra bearer-style check for the MCP port (enforcement can be added in a transport layer if your deployment requires it).
- Combine with TOON responses (`LlmResponse<T>`) if you want token-efficient data for the agent (see the [AI Integration (TOON)](ai_integration.md) recipe).

## Permission Scoping (Important for Agents)

RustAPI now has **framework-native permission scoping** for MCP. By default `ToolPolicy::ReadOnly` is used so only read operations are exposed to agents.

### Route-level `#[mcp(...)]` annotations

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::protocol::mcp::ToolPolicy;

// Read-only by nature (GET)
#[rustapi_rs::get("/weather/{city}")]
#[rustapi_rs::tag("agent")]
async fn get_weather(...) -> ... { ... }

// Write operation that requires user confirmation in the agent UI
#[rustapi_rs::post("/orders")]
#[rustapi_rs::tag("agent")]
#[rustapi_rs::mcp(write, require = "confirm")]
async fn create_order(Json(body): Json<CreateOrder>) -> ... { ... }

// Never expose this to agents
#[rustapi_rs::get("/admin/secrets")]
#[rustapi_rs::mcp(skip)]
async fn admin_secrets() -> ... { ... }

// This POST is actually safe (idempotent webhook)
#[rustapi_rs::post("/webhooks/stripe")]
#[rustapi_rs::mcp(readonly)]
async fn stripe_webhook() -> ... { ... }
```

### Server-level policy

```rust
let mcp = McpServer::from_rustapi(
    &app,
    McpConfig::new()
        .allowed_tags(["agent"])
        .tool_policy(ToolPolicy::ReadOnly),   // Safe default
        // .tool_policy(ToolPolicy::All),
);
```

### Metadata exposed to agents (`tools/list`)

```json
{
  "name": "createOrder",
  "description": "Create a new order",
  "inputSchema": { ... },
  "permission": "write",
  "requiresConfirmation": true
}
```

Agents (Claude, Cursor, etc.) can use the `permission` and `requiresConfirmation` fields to decide whether to auto-approve or ask the user.

The metadata comes from the `x-mcp` OpenAPI extension that the `#[mcp(...)]` attribute populates on the operation. This keeps everything inside the normal OpenAPI document.

### ReadOnly vs All policies

```rust
// Safe default for agents â€“ only GET/HEAD etc. become tools
.tool_policy(ToolPolicy::ReadOnly)

// Full access (writes will appear)
.tool_policy(ToolPolicy::All)
```

A `#[post(...)]` or `#[delete(...)]` is automatically hidden when `ReadOnly` is active (unless you explicitly mark it `#[mcp(readonly)]`).

## Testing Your MCP Surface

The `rustapi-mcp` crate ships with helpers that make it easy to test the sidecar in-process during normal `cargo test`.

See the integration tests in the repository (`crates/rustapi-mcp/tests/mcp_e2e.rs`) for patterns using ephemeral ports + `reqwest` against the JSON-RPC endpoint while the full `run_rustapi_and_mcp_with_shutdown` is active.

## Further Reading

- [MCP In-Process Invocation](mcp_in_process.md)
- [OpenAPI to MCP CLI](mcp_openapi_cli.md)
- [MCP stdio Transport](mcp_stdio.md)
- Full standalone **MCP tool example** (05-mcp-server with in-process mode): [rustapi-rs-examples](https://github.com/Tuntii/rustapi-rs-examples/tree/main/05-mcp-server) â€” see also the quick internal demo in this repo at `crates/rustapi-rs/examples/mcp_tools.rs`
- Native MCP plan: `memories/native_mcp_orchestration_plan.md`
- gRPC side-by-side pattern: [gRPC Integration](grpc_integration.md)
- TOON for LLM efficiency: [AI Integration (TOON)](ai_integration.md)
