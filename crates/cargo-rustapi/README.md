# cargo-rustapi

**Lens**: "The Architect"  
**Philosophy**: "Scaffolding best practices from day one."

The RustAPI CLI isn't just a project generator; it's a productivity multiplier for scaffolding, development, deployment, MCP tooling, and RustAPI Cloud integration.

## Installation

```bash
cargo install cargo-rustapi
```

## Commands

### Project lifecycle

| Command | Description |
|---------|-------------|
| `cargo rustapi new <name>` | Create a new project with the perfect directory structure |
| `cargo rustapi new <name> --preset prod-api` | Production middleware bundle |
| `cargo rustapi new <name> --preset ai-api` | TOON + AI-oriented defaults |
| `cargo rustapi new <name> --preset realtime-api` | WebSocket defaults |
| `cargo rustapi run` | Run the development server |
| `cargo rustapi run --reload` | Run with hot-reload (auto-rebuild on file changes) |

### Code generation & tooling

| Command | Description |
|---------|-------------|
| `cargo rustapi generate resource <name>` | Scaffold a new API resource (Model + Handlers + Tests) |
| `cargo rustapi client --spec <path> --language <lang>` | Generate a client library (Rust, TS, Python) from OpenAPI spec |
| `cargo rustapi mcp generate --spec <file\|url> --target <backend>` | Turn any OpenAPI spec into a live MCP server |
| `cargo rustapi migrate <action>` | Database migration commands (create, run, revert, status, reset) |

### Deployment

| Command | Description |
|---------|-------------|
| `cargo rustapi deploy docker` | Generate a production `Dockerfile` |
| `cargo rustapi deploy fly` | Generate Fly.io config |
| `cargo rustapi deploy railway` | Generate Railway config |
| `cargo rustapi deploy shuttle` | Generate Shuttle.rs config |
| `cargo rustapi deploy cloud` | Deploy to RustAPI Cloud (managed hosting) |
| `cargo rustapi deploy status <id>` | Poll cloud deploy job progress |

### RustAPI Cloud auth

| Command | Description |
|---------|-------------|
| `cargo rustapi login` | Device-code OAuth (default: `https://api.rustapi.cloud`) |
| `cargo rustapi login --cloud-url <url>` | Self-hosted cloud backend |
| `cargo rustapi whoami` | Show logged-in user and tier |
| `cargo rustapi logout` | Clear local credentials |

Credentials: `~/.rustapi/config.json` (override with `RUSTAPI_CONFIG_PATH`).

Cloud backend source: [github.com/Tuntii/RustAPI-Cloud](https://github.com/Tuntii/RustAPI-Cloud)

### Operations

| Command | Description |
|---------|-------------|
| `cargo rustapi doctor [--strict]` | Validate toolchain and production signals |
| `cargo rustapi observability [--check]` | Observability docs and recommended features |
| `cargo rustapi bench` | Run benchmark workflow |
| `cargo rustapi replay <subcommand>` | Time-travel replay from a running service |

## Quick Start

```bash
# Create a new project
cargo rustapi new my-app --template api --preset prod-api

# Run with auto-reload
cd my-app
cargo rustapi run --reload
```

## Cloud deploy

```bash
cargo rustapi login
cargo rustapi deploy cloud
cargo rustapi deploy status <deploy-id>
```

Full guide: [docs/cookbook/src/recipes/rustapi_cloud.md](../../docs/cookbook/src/recipes/rustapi_cloud.md)

## Templates

The templates used by the CLI are opinionated but flexible. They enforce:

- Modular folder structure
- Implementation of `State` pattern
- Separation of `Error` types

**Available Templates:**

- `minimal`: Basic `main.rs` and `Cargo.toml`
- `api`: REST API structure with separated `handlers` and `models`
- `web`: Web application with HTML templates (`rustapi-view`)
- `full`: Complete example with Database, Auth, and Docker support

**Available Presets:**

- `prod-api`: production-facing API defaults (`extras-config`, `extras-cors`, `extras-rate-limit`, `extras-security-headers`, `extras-structured-logging`, `extras-timeout`)
- `ai-api`: AI-oriented API defaults with `protocol-toon`
- `realtime-api`: realtime-oriented API defaults with `protocol-ws`

## Feature flag: `cloud`

Cloud HTTP commands are enabled by default. Disable with:

```toml
[dependencies]
cargo-rustapi = { version = "0.1.550", default-features = false }
```