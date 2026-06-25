# cargo-rustapi: The Architect

**Lens**: "The Architect"  
**Philosophy**: "Scaffolding best practices from day one."

The RustAPI CLI (`cargo-rustapi`) is the productivity layer on top of the framework: project scaffolding, dev server, deployment, MCP tooling, observability helpers, and RustAPI Cloud integration.

## Installation

```bash
cargo install cargo-rustapi
```

From source (this repository):

```bash
cargo install --path crates/cargo-rustapi
```

---

## Command reference

### Project lifecycle

| Command | Description |
|---------|-------------|
| `cargo rustapi new <name>` | Create a new project |
| `cargo rustapi new <name> --template api` | REST API layout (handlers + models) |
| `cargo rustapi new <name> --template web` | Web app with `rustapi-view` templates |
| `cargo rustapi new <name> --template full` | Database, auth, Docker scaffolding |
| `cargo rustapi new <name> --preset prod-api` | Production middleware bundle |
| `cargo rustapi new <name> --preset ai-api` | TOON + AI-oriented defaults |
| `cargo rustapi new <name> --preset realtime-api` | WebSocket defaults |
| `cargo rustapi run` | Run the dev server |
| `cargo rustapi run --reload` | Hot-reload via `cargo-watch` |
| `cargo rustapi watch` | Alias for reload mode |

### Code generation

| Command | Description |
|---------|-------------|
| `cargo rustapi generate resource <name>` | Scaffold model + handlers + tests |
| `cargo rustapi client --spec <path> --language rust` | Client from OpenAPI (Rust, TS, Python) |
| `cargo rustapi migrate create <name>` | Create SQL migration |
| `cargo rustapi migrate run` | Apply pending migrations |

### MCP & AI agents

| Command | Description |
|---------|-------------|
| `cargo rustapi mcp generate --spec <file\|url> --target <backend>` | OpenAPI → live MCP server |
| `cargo rustapi mcp stdio` | Run MCP over stdio (Claude Desktop, Cursor) |

See [MCP Integration](../recipes/mcp_integration.md), [MCP In-Process](../recipes/mcp_in_process.md), and [OpenAPI to MCP CLI](../recipes/mcp_openapi_cli.md).

### Deployment

| Command | Description |
|---------|-------------|
| `cargo rustapi deploy docker` | Generate production `Dockerfile` |
| `cargo rustapi deploy fly` | Generate `fly.toml` |
| `cargo rustapi deploy railway` | Generate Railway config |
| `cargo rustapi deploy shuttle` | Generate Shuttle.rs config |
| `cargo rustapi deploy cloud` | Deploy to RustAPI Cloud (managed) |
| `cargo rustapi deploy status <id>` | Poll cloud deploy job |

Self-hosted platforms: [Deployment recipe](../recipes/deployment.md).  
Managed hosting: [RustAPI Cloud recipe](../recipes/rustapi_cloud.md).

### RustAPI Cloud auth

| Command | Description |
|---------|-------------|
| `cargo rustapi login` | Device-code OAuth (default: `https://api.rustapi.cloud`) |
| `cargo rustapi login --cloud-url <url>` | Point at self-hosted cloud backend |
| `cargo rustapi login --no-browser` | Print verification URL only |
| `cargo rustapi whoami` | Show logged-in user and tier |
| `cargo rustapi logout` | Clear local credentials |

Credentials stored at `~/.rustapi/config.json` (override with `RUSTAPI_CONFIG_PATH`).

### Operations & debugging

| Command | Description |
|---------|-------------|
| `cargo rustapi doctor [--strict]` | Toolchain + production signal scan |
| `cargo rustapi observability [--check]` | Observability docs and feature recommendations |
| `cargo rustapi bench` | Run benchmark workflow |
| `cargo rustapi replay list -t <token>` | List captured replay entries |
| `cargo rustapi replay run <id> -t <token>` | Replay a captured request |
| `cargo rustapi replay diff <id> -t <token>` | Diff replay against target URL |

See [Replay recipe](../recipes/replay.md) and [Production Checklist](../../PRODUCTION_CHECKLIST.md).

---

## Quick start

```bash
cargo rustapi new my-api --template api --preset prod-api
cd my-api
cargo rustapi run --reload
```

Open `http://127.0.0.1:8080/docs` for Swagger UI.

---

## Cloud deploy workflow

```bash
cargo rustapi login
cargo rustapi whoami
cargo rustapi deploy cloud
cargo rustapi deploy status <deploy-id>
```

Backend source: [github.com/Tuntii/RustAPI-Cloud](https://github.com/Tuntii/RustAPI-Cloud)

---

## Templates

Templates enforce:

- Modular folder structure
- `State` pattern for shared resources
- Separated error types

| Template | Best for |
|----------|----------|
| `minimal` | Smallest possible `main.rs` |
| `api` | REST APIs |
| `web` | Server-rendered HTML |
| `full` | DB + auth + Docker |

---

## Feature flag: `cloud`

Cloud commands require the `cloud` feature (default-on):

```toml
[dependencies]
cargo-rustapi = { version = "0.1.550", default-features = false }
```

Disabling removes `login`, `deploy cloud`, and `deploy status` — useful for minimal CI builds.

---

## Related

- [RustAPI Cloud recipe](../recipes/rustapi_cloud.md)
- [Deployment recipe](../recipes/deployment.md)
- [Getting Started: Installation](../getting_started/installation.md)