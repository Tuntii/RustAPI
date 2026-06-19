# OpenAPI to MCP CLI

The `cargo-rustapi` CLI includes `rustapi mcp generate`. It takes **any** OpenAPI 3.x spec (from FastAPI, Express, Go, Spring, etc.) and instantly spins up an MCP server. Tool calls are proxied to the real backend.

This is a major growth feature: non-Rust teams can get first-class agent tools with zero Rust code.

## Installation

```bash
cargo install cargo-rustapi
# or from source
cargo install --path crates/cargo-rustapi
```

## Basic Usage

```bash
# From a local spec file
rustapi mcp generate \
  --spec ./openapi.json \
  --target http://localhost:8000 \
  --port 9090 \
  --tags public,agent

# From a URL
rustapi mcp generate \
  --url https://api.example.com/openapi.json \
  --target https://api.example.com \
  --port 9090

# Point at a running service (auto-fetches /openapi.json)
rustapi mcp generate \
  --api http://localhost:8080 \
  --port 9090
```

## Flags

| Flag                  | Description |
|-----------------------|-------------|
| `--spec <FILE>`       | Local JSON or YAML OpenAPI file |
| `--url <URL>`         | Fetch spec from HTTP URL |
| `--api <BASE>`        | Use `<BASE>/openapi.json` as spec source + default target |
| `--target <URL>`      | Backend that tool calls proxy to (required unless `--api`) |
| `--port <N>`          | MCP listen port (default 9090) |
| `--name <NAME>`       | Server name advertised to clients |
| `--tags <TAGS>`       | Comma-separated tags to expose |
| `--allow-path-prefix` | Only include paths starting with prefix |
| `--stdio`             | Use stdio transport instead of HTTP |

## Example Walkthrough with a Python FastAPI

1. Start your FastAPI app on port 8000 (it serves OpenAPI at `/openapi.json`).

2. In another terminal:

```bash
rustapi mcp generate --api http://localhost:8000 --port 9090 --tags public
```

3. Test with curl or connect Claude Desktop / Cursor to `http://localhost:9090`.

4. Agents see your FastAPI endpoints as tools. Calls go through the real backend (auth, validation, DB, etc.).

## How It Works

- Uses `McpServer::from_spec(config, &parsed_openapi)`.
- Reuses the same discovery and proxy logic as native RustAPI MCP.
- No RustAPI server required on the target side.

## Security Note

The generated MCP server is only as secure as the filters you apply (`--tags`, `--allow-path-prefix`). Never expose internal/admin routes to agents unless you intend to.

## Related

- Native MCP: [MCP Integration](mcp_integration.md)
- In-process: [MCP In-Process Invocation](mcp_in_process.md)
- stdio: [MCP stdio Transport](mcp_stdio.md)