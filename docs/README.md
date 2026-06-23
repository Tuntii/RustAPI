# RustAPI Documentation

Central index for user guides, architecture notes, and open-source contribution paths.

**Current release:** [`rustapi-rs` 0.1.537](https://crates.io/crates/rustapi-rs) Â· [Changelog](../CHANGELOG.md) Â· [All releases](https://github.com/Tuntii/RustAPI/releases)

---

## Start here

| Document | Description |
|----------|-------------|
| [Getting Started](GETTING_STARTED.md) | First API in ~5 minutes |
| [Cookbook](cookbook/src/SUMMARY.md) | Recipes, crate deep dives, learning paths |
| [Features](FEATURES.md) | Feature reference |
| [Community & Contributing](COMMUNITY.md) | How to get help, contribute, and report issues |

## Architecture & design

| Document | Description |
|----------|-------------|
| [Philosophy](PHILOSOPHY.md) | Design principles |
| [Architecture](ARCHITECTURE.md) | Internal structure |
| [Performance Benchmarks](PERFORMANCE_BENCHMARKS.md) | Methodology and published claims |
| [Public API Contract](../CONTRACT.md) | Stability and semver expectations |

## Production

| Document | Description |
|----------|-------------|
| [Production Baseline](PRODUCTION_BASELINE.md) | Recommended defaults |
| [Production Checklist](PRODUCTION_CHECKLIST.md) | Rollout checklist |
| [Cookbook: Deployment](cookbook/src/recipes/deployment.md) | Deploy patterns |
| [Cookbook: Observability](cookbook/src/recipes/observability.md) | Metrics, tracing, health |
| [Cookbook: Graceful Shutdown](cookbook/src/recipes/graceful_shutdown.md) | Clean shutdown |
| [Cookbook: Replay](cookbook/src/recipes/replay.md) | Request capture and replay |

## Open source

| Document | Description |
|----------|-------------|
| [Contributing](../CONTRIBUTING.md) | Dev setup, tests, PR process |
| [Code of Conduct](../CODE_OF_CONDUCT.md) | Community standards |
| [Security](../SECURITY.md) | Vulnerability reporting |
| [Community guide](COMMUNITY.md) | Channels, doc locations, release cadence |

## Examples

- In-repo: [`crates/rustapi-rs/examples/`](../crates/rustapi-rs/examples/README.md)
- Full projects: [rustapi-rs-examples](https://github.com/Tuntii/rustapi-rs-examples)

## Quick start

```toml
[dependencies]
rustapi-rs = "0.1.537"
```

Alias for shorter macros (recommended):

```toml
[dependencies]
api = { package = "rustapi-rs", version = "0.1.537" }
```

```rust
use api::prelude::*;

#[derive(Serialize, Schema)]
struct Message { text: String }

#[api::get("/hello/{name}")]
async fn hello(Path(name): Path<String>) -> Json<Message> {
    Json(Message { text: format!("Hello, {name}!") })
}

#[api::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::auto().run("127.0.0.1:8080").await
}
```

Open `http://127.0.0.1:8080/docs` for auto-generated OpenAPI / Swagger UI.

## Planned work

Design and planning docs (not yet shipped):

- [GraphQL Adapter Plan](GRAPHQL_ADAPTER_PLAN.md)
- [Adaptive Execution Debug Plan](ADAPTIVE_EXECUTION_DEBUG_PLAN.md)

## License

MIT OR Apache-2.0, at your option.