# RustAPI Documentation

Central index for user guides, architecture notes, and open-source contribution paths.

**Current release:** [`rustapi-rs` 0.1.550](https://crates.io/crates/rustapi-rs) · [Changelog](../CHANGELOG.md) · [All releases](https://github.com/Tuntii/RustAPI/releases)

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
| [Production Baseline](PRODUCTION_BASELINE.md) | Recommended defaults (`production_defaults`, probes, middleware) |
| [Production Checklist](PRODUCTION_CHECKLIST.md) | Pre-deploy and rollout checklist |
| [Cookbook: Deployment](cookbook/src/recipes/deployment.md) | Docker, Fly.io, Railway, Shuttle, K8s |
| [Cookbook: RustAPI Cloud](cookbook/src/recipes/rustapi_cloud.md) | Managed hosting via CLI |
| [Cookbook: Observability](cookbook/src/recipes/observability.md) | Metrics, tracing, health |
| [Cookbook: Graceful Shutdown](cookbook/src/recipes/graceful_shutdown.md) | Clean shutdown |
| [Cookbook: Replay](cookbook/src/recipes/replay.md) | Request capture and replay |

## CLI & tooling

| Document | Description |
|----------|-------------|
| [cargo-rustapi (Cookbook)](cookbook/src/crates/cargo_rustapi.md) | Full command reference |
| [RustAPI Cloud backend](https://github.com/Tuntii/RustAPI-Cloud) | Self-hosted / managed cloud source |

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
rustapi-rs = "0.1.550"
```

Alias for shorter macros (recommended):

```toml
[dependencies]
api = { package = "rustapi-rs", version = "0.1.550" }
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

## Deploy to RustAPI Cloud

```bash
cargo install cargo-rustapi
cargo rustapi login
cargo rustapi deploy cloud
```

See [RustAPI Cloud recipe](cookbook/src/recipes/rustapi_cloud.md) for auth, status polling, and self-hosted backends.

## License

MIT OR Apache-2.0, at your option.