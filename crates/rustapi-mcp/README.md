# rustapi-mcp

Native [Model Context Protocol (MCP)](https://modelcontextprotocol.io) integration for RustAPI.

This crate allows you to expose your existing RustAPI routes as **discoverable tools** for LLMs and AI agents (Claude, Cursor, custom agents, etc.) with full respect for your middleware, validation, auth, and observability layers.

## Status

**Core implementation complete** (discovery, `tools/list`, real `tools/call` via proxy into normal RustAPI pipeline, HTTP JSON-RPC transport, concurrent runner with `run_rustapi_and_mcp`).

HTTP transport enforces `McpConfig::admin_token` when set (`Authorization: Bearer`,
`X-MCP-Token`, or `?token=`). Non-path tool arguments are forwarded as query
parameters on safe HTTP methods.

## Goals (v1)

- Opt-in via `protocol-mcp` feature
- Automatic tool discovery from your route metadata + OpenAPI schemas
- Tool invocations execute through the **normal** RustAPI request pipeline (no bypass)
- Support for the recommended HTTP + SSE transport (stdio later)
- Strong security defaults (explicit exposure control, never auto-expose everything)

## Usage (planned shape)

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::protocol::mcp::{McpConfig, McpServer};

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<u64>) -> Json<User> { /* ... */ }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = RustApi::auto();

    // Run your normal HTTP API + MCP side-by-side (similar to gRPC)
    let mcp = McpServer::new(
        McpConfig::new()
            .name("my-api")
            .version("1.0.0")
            .enable_tools(true)
            // Control what gets exposed as tools
            .allowed_tags(["public", "agent"])
            // When set, every MCP HTTP request must present this token
            .admin_token("secret-for-mcp-clients"),
    );

    rustapi_mcp::run_rustapi_and_mcp(app, "0.0.0.0:8080", mcp, "0.0.0.0:9090").await?;
    Ok(())
}
```

Clients send the token as:

```http
Authorization: Bearer secret-for-mcp-clients
```

or `X-MCP-Token: secret-for-mcp-clients`, or `?token=secret-for-mcp-clients`.

## Feature Flag

In `rustapi-rs`:

```toml
[dependencies]
rustapi-rs = { version = "...", features = ["protocol-mcp"] }
```

Or with the meta feature:

```toml
rustapi-rs = { version = "...", features = ["protocol-all"] }
```

## Design Principles

- Follows the same facade contract as the rest of RustAPI.
- New internal crate (`rustapi-mcp`) behind the `protocol-mcp` feature.
- Tool calls **must** go through existing `Router`, layers, interceptors, extractors, and validation.
- Explicit exposure model by default (no accidental leakage of internal routes).

## Related

- Main roadmap: see top-level `README.md`
- Detailed plan: `memories/native_mcp_orchestration_plan.md`
- Master task list: `memories/TASKLIST.md`

## License

MIT OR Apache-2.0 (same as the rest of the workspace)
