# MCP In-Process Invocation

By default, `tools/call` from an MCP server proxies over HTTP to your main RustAPI server (even on localhost). This guarantees that every middleware, interceptor, extractor, and validator runs exactly as for normal traffic.

For high-frequency agent use (many tool calls per prompt), the network/serde overhead of the proxy can add up. RustAPI supports an optional **in-process** invocation path that constructs a `Request` and drives it directly through the `Router` + `LayerStack` with zero TCP or serialization.

## When to Use In-Process

- You are using `run_rustapi_and_mcp` (or equivalent) so the MCP sidecar and main HTTP server are in the **same process**.
- You make many sequential or batched tool calls from agents.
- You want the absolute lowest latency while keeping the full pipeline.

**Do not** use in-process if your MCP server talks to a *remote* RustAPI instance (use the proxy).

## How to Enable

```rust
use rustapi_rs::protocol::mcp::{McpConfig, McpServer, InvocationMode};

let mcp = McpServer::from_rustapi(
    &app,
    McpConfig::new()
        .allowed_tags(["agent"])
        .invocation_mode(InvocationMode::InProcess),  // or Auto
);
```

Modes:
- `Proxy` (default): always use HTTP proxy.
- `InProcess`: use direct dispatch (requires `from_rustapi`).
- `Auto`: prefer in-process when a `RustApi` was attached, fall back to proxy.

When you use `run_rustapi_and_mcp*`, the runner automatically wires the HTTP base for the proxy path. In-process mode simply ignores it.

## Performance

See the benchmark in `crates/rustapi-mcp/tests/mcp_e2e.rs`.

Typical numbers on a dev machine (1000 sequential tool calls):

- In-process: ~28 Âµs per call
- Proxy (live localhost HTTP): ~1.3 ms per call
- Speedup: ~45-50x

The in-process path still executes the full middleware chain, extractors, validation, and error handling.

## Implementation Notes

The `RequestDispatcher` (obtained via `app.request_dispatcher()`) clones the necessary `Arc<Router>`, `LayerStack`, and `InterceptorChain`. Your `McpServer` stores a `RequestInvoker` when created via `from_rustapi`.

Tool calls still go through:
- Request interceptors
- Middleware layers (in order)
- Router match + handler (with extractors, Schema validation, etc.)
- Response interceptors

No shortcuts are taken.

## Cookbook Cross-References

- Main MCP page: [MCP Integration (Agent Tools)](mcp_integration.md)
- CLI generator: [OpenAPI to MCP CLI](mcp_openapi_cli.md)
- stdio: [MCP stdio Transport](mcp_stdio.md)