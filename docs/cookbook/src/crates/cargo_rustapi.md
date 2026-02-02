# cargo-rustapi: The Architect

**Lens**: "The Architect"
**Philosophy**: "Scaffolding best practices from day one."

## The CLI

The RustAPI CLI isn't just a project generator; it's a productivity multiplier.

### Commands

- `cargo rustapi new <name>`: Create a new project with the perfect directory structure.
- `cargo rustapi run`: Run the development server.
- `cargo rustapi run --reload`: Run with hot-reload (auto-rebuild on file changes).
- `cargo rustapi generate resource <name>`: Scaffold a new API resource (Model + Handlers + Tests).
- `cargo rustapi client --spec <path> --language <lang>`: Generate a client library (Rust, TS, Python) from OpenAPI spec.
- `cargo rustapi deploy <platform>`: Generate deployment configs for Docker, Fly.io, Railway, or Shuttle.
- `cargo rustapi migrate <action>`: Database migration commands (create, run, revert, status, reset).

## Templates

The templates used by the CLI are opinionated but flexible. They enforce:
- Modular folder structure.
- Implementation of `State` pattern.
- Separation of `Error` types.
