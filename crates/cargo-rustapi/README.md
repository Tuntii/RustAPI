# cargo-rustapi

**Lens**: "The Architect"  
**Philosophy**: "Scaffolding best practices from day one."

The RustAPI CLI isn't just a project generator; it's a productivity multiplier.

## 📦 Installation

```bash
cargo install cargo-rustapi
```

## 🛠️ Commands

| Command | Description |
|---------|-------------|
| `cargo rustapi new <name>` | Create a new project with the perfect directory structure |
| `cargo rustapi run` | Run the development server |
| `cargo rustapi run --reload` | Run with hot-reload (auto-rebuild on file changes) |
| `cargo rustapi bench` | Run the repository benchmark workflow via `scripts/bench.ps1` |
| `cargo rustapi doctor [--strict]` | Validate toolchain availability and check project signals against the production checklist |
| `cargo rustapi observability [--check]` | Surface observability docs, benchmark assets, and recommended baseline features |
| `cargo rustapi new <name> --preset <preset>` | Start from opinionated `prod-api`, `ai-api`, or `realtime-api` feature bundles |
| `cargo rustapi generate resource <name>` | Scaffold a new API resource (Model + Handlers + Tests) |
| `cargo rustapi client --spec <path> --language <lang>` | Generate a client library (Rust, TS, Python) from OpenAPI spec |
| `cargo rustapi deploy <platform>` | Generate deployment configs for Docker, Fly.io, Railway, or Shuttle |
| `cargo rustapi migrate <action>` | Database migration commands (create, run, revert, status, reset) |
| `cargo rustapi replay <subcommand>` | Work with time-travel replay entries from a running RustAPI service |

## 🚀 Quick Start

```bash
# Create a new project
cargo rustapi new my-app --template api

# Run with auto-reload
cd my-app
cargo rustapi run --reload
```

## 📁 Templates

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
