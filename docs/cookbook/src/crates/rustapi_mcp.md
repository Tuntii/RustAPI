# rustapi-mcp: The Agent Bridge

**Lens**: "Your API is now a tool provider for LLMs."  
**Philosophy**: "Discover once, execute through the real stack, expose nothing by accident."

`rustapi-mcp` turns a normal RustAPI application into a first-class participant in the Model Context Protocol (MCP) ecosystem. Compatible clients (Claude Desktop, Cursor, custom agent runtimes, etc.) can discover a curated set of your endpoints as tools and invoke them with full respect for your existing middleware, validation, and observability.

## What You Get

- `McpConfig` builder for name, version, `allowed_tags`, `allowed_path_prefixes`, and `admin_token`.
- `McpServer::new(...)`, `::from_rustapi(&app, config)`, `::from_spec(...)`.
- Automatic conversion of OpenAPI operations (powered by your `#[derive(Schema)]` types) into MCP tool manifests (`name`, `description`, `inputSchema`).
- Real `tools/call` execution via a sidecar HTTP JSON-RPC server. Calls are proxied back to your main RustAPI listener so every layer runs exactly as for normal traffic.
- `run_rustapi_and_mcp` + `run_rustapi_and_mcp_with_shutdown` helpers (same shape as the gRPC ones).
- Feature flag: `protocol-mcp` (also included in `protocol-all` and `full`).

## Enable It

```toml
[dependencies]
rustapi-rs = { version = "0.1.478", features = ["protocol-mcp"] }
```

## Basic Usage (Recommended)

```rust,ignore
use rustapi_rs::prelude::*;
use rustapi_rs::protocol::mcp::{McpConfig, McpServer, run_rustapi_and_mcp};

let app = RustApi::auto();

let mcp = McpServer::from_rustapi(
    &app,
    McpConfig::new()
        .name("my-api")
        .allowed_tags(["public", "agent"]),
);

run_rustapi_and_mcp(app, "0.0.0.0:8080", mcp, "0.0.0.0:9090").await?;
```

Tool calls arriving on port 9090 are turned into real HTTP requests against port 8080 (or whichever address you configured). This guarantees that JWT layers, rate limiters, circuit breakers, request replay recording, etc. all see the invocation.

## Security Model

Nothing is auto-exposed. The two main controls live in `McpConfig`:

- `allowed_tags(...)` â€” only operations carrying at least one of the listed tags become tools.
- `allow_path_prefix(...)` â€” additional path-based allow list.

This is the same philosophy as the rest of RustAPI: explicit and auditable.

## Current Transport

MCP v1 uses a minimal JSON-RPC over plain HTTP POST (the same style used by many early MCP implementations). The crate is structured so additional transports (full SSE, stdio bridge) can be added later behind the same `McpServer` surface.

## Relationship to Other Crates

- Reuses `rustapi-openapi` for schema and operation metadata (no duplication).
- Works beautifully with `protocol-toon` / `LlmResponse` when you want token-efficient payloads for agents.
- Plays well with `extras-replay`, `extras-audit`, and all the resilience layers because tool calls are real traffic.

## Status & Roadmap

Core functionality (discovery + real proxied invocation + runner) is complete. See the top-level README and `memories/native_mcp_orchestration_plan.md` for remaining polish items (cookbook examples, more client conformance tests, optional direct in-process invoker to avoid the localhost hop).

## See Also

- Recipe: [MCP Integration (Agent Tools)](../recipes/mcp_integration.md)
- gRPC counterpart: [rustapi-grpc](rustapi_grpc.md)
- AI-friendly responses: [rustapi-toon](rustapi_toon.md)
