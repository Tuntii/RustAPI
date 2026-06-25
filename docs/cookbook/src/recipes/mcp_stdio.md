# MCP stdio Transport

In addition to the HTTP JSON-RPC transport, `rustapi mcp generate` supports `--stdio`.

stdio is the transport used by many local AI clients (Claude Desktop, some Cursor setups, custom agents) that spawn the MCP server as a child process and communicate over stdin/stdout.

## Usage

```bash
rustapi mcp generate \
  --spec ./openapi.json \
  --target http://localhost:8000 \
  --stdio
```

When `--stdio` is passed, the command does **not** bind a TCP port. Instead it enters a loop:

- Reads JSON-RPC lines from stdin
- Dispatches `initialize`, `tools/list`, `tools/call`
- Writes JSON-RPC responses to stdout

## Why stdio?

- No network port to manage or firewall.
- Simple process model for desktop apps.
- Works well with local-only tools.

## Current Limitations (MVP)

- Only basic JSON-RPC over line-delimited messages (no SSE framing yet).
- No built-in auth/token for stdio (rely on OS process isolation + tool-level filters).
- Logging goes to stderr.

## Connecting Clients

Most clients that support "local MCP server" or "command" transport can use:

```json
{
  "command": "rustapi",
  "args": ["mcp", "generate", "--spec", "/path/to/openapi.json", "--target", "http://127.0.0.1:8000", "--stdio"]
}
```

See your client's documentation for exact config format.

## Related Pages

- Main MCP: [MCP Integration](mcp_integration.md)
- In-process: [MCP In-Process Invocation](mcp_in_process.md)
- CLI generator: [OpenAPI to MCP CLI](mcp_openapi_cli.md)