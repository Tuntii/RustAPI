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
| `cargo rustapi generate resource <name>` | Scaffold a new API resource (Model + Handlers + Tests) |
| `cargo rustapi client --spec <path> --language <lang>` | Generate a client library (Rust, TS, Python) from OpenAPI spec |
| `cargo rustapi deploy <platform>` | Generate deployment configs for Docker, Fly.io, Railway, or Shuttle |
| `cargo rustapi migrate <action>` | Database migration commands (create, run, revert, status, reset) |

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
