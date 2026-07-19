# Golden Path

**One story:** define a handler → get OpenAPI → run probes → optional MCP / deploy.

Everything else (JWT, jobs, WS, gRPC, full extras) is a branch off this path. Start here.

| Step | What you get |
|------|----------------|
| 1. Handler + `Schema` | Typed JSON response |
| 2. `RustApi::auto()` | Route discovery, Swagger UI `/docs`, spec `/docs/openapi.json` |
| 3. `production_defaults` | Request IDs, tracing spans, `/live` `/ready` `/health` |
| 4. (optional) MCP | Same routes as agent tools |
| 5. (optional) Cloud / Docker | Ship the binary |

---

## 60 seconds (in this repo)

```bash
git clone https://github.com/Tuntii/RustAPI.git
cd RustAPI
cargo run -p rustapi-rs --example golden_path
```

In another terminal:

```bash
curl -s http://127.0.0.1:8080/api/v1/ping
# {"status":"ok"}

curl -s http://127.0.0.1:8080/live
# healthy probe JSON

# OpenAPI paths include /api/v1/ping
curl -s http://127.0.0.1:8080/docs/openapi.json | head -c 400

# Browser
# http://127.0.0.1:8080/docs
```

Source: [`crates/rustapi-rs/examples/golden_path.rs`](../crates/rustapi-rs/examples/golden_path.rs)

---

## New project (crates.io)

```bash
cargo new hello-rustapi && cd hello-rustapi
cargo add rustapi-rs
cargo add tokio --features macros,rt-multi-thread
cargo add tracing-subscriber --features env-filter
```

`src/main.rs`:

```rust
use rustapi_rs::prelude::*;
use tokio::signal;

#[derive(Debug, Serialize, Schema)]
struct PingResponse {
    status: &'static str,
}

#[rustapi_rs::get("/api/v1/ping")]
#[rustapi_rs::tag("public")]
#[rustapi_rs::summary("Ping")]
async fn ping() -> Json<PingResponse> {
    Json(PingResponse { status: "ok" })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    RustApi::auto()
        .production_defaults("hello-rustapi")
        .run_with_shutdown("127.0.0.1:8080", async {
            signal::ctrl_c().await.ok();
        })
        .await
}
```

```bash
cargo run
curl -s http://127.0.0.1:8080/api/v1/ping
# open http://127.0.0.1:8080/docs
```

**Crate alias (optional):** shorter macros like FastAPI-style imports:

```toml
[dependencies]
api = { package = "rustapi-rs", version = "0.1" }
```

```rust
use api::prelude::*;

#[api::get("/api/v1/ping")]
#[api::tag("public")]
async fn ping() -> Json<PingResponse> { /* ... */ }
```

Prefer **slim** features in real services (`default-features = false, features = ["core"]`) — see [Getting Started](GETTING_STARTED.md).

---

## Why macros, not only `.route()`

`#[rustapi_rs::get("/...")]` registers the handler at link time **and** feeds the OpenAPI operation (path, tags, response schemas).

`.route(path, get(handler))` is valid for dynamic wiring, but the golden path uses macros so `/docs` is not an empty shell.

---

## What `production_defaults` turns on

| Capability | Endpoints / behavior |
|------------|----------------------|
| Request IDs | `x-request-id` on responses |
| Tracing | Per-request spans (with your subscriber) |
| Probes | `GET /live`, `GET /ready`, `GET /health` |
| Service name | Attached to logs / health metadata |

Details: [Production Baseline](PRODUCTION_BASELINE.md)  
Ship checklist: [Production Checklist](PRODUCTION_CHECKLIST.md)

Set `RUSTAPI_ENV=production` to mask 5xx detail bodies while keeping `error_id` for log correlation.

---

## Next fork: MCP (agent tools)

Expose the same tagged routes as MCP tools (feature `protocol-mcp`):

```rust
use rustapi_rs::protocol::mcp::{McpConfig, McpServer, run_rustapi_and_mcp_with_shutdown};

// after building the same auto() app pattern — see mcp_tools example
let mcp = McpServer::from_rustapi(&app, McpConfig::new().allowed_tags(["public"]));
```

In-repo: `cargo run -p rustapi-rs --example mcp_tools --features protocol-mcp`  
Cookbook: [MCP Integration](cookbook/src/recipes/mcp_integration.md) (if present) · crate notes under cookbook `rustapi_mcp`

CLI (any OpenAPI → MCP):

```bash
cargo install cargo-rustapi
cargo rustapi mcp generate --help
```

---

## Next fork: deploy

**Local binary / container:** [Deployment recipe](cookbook/src/recipes/deployment.md)

**Managed:**

```bash
cargo install cargo-rustapi
cargo rustapi login
cargo rustapi deploy cloud
```

Guide: [RustAPI Cloud](cookbook/src/recipes/rustapi_cloud.md)

---

## What not to do on day one

- Enable `full` feature for production compiles
- Skip probes and ship behind a load balancer
- Treat every recipe (JWT, jobs, WS) as required core
- Promise HTTP/2/3 edge behavior without reading [Performance Benchmarks](PERFORMANCE_BENCHMARKS.md) methodology

---

## Related

| Doc | Role |
|-----|------|
| [Getting Started](GETTING_STARTED.md) | Longer tutorial + feature table |
| [Cookbook Quickstart](cookbook/src/getting_started/quickstart.md) | CLI `cargo rustapi new` path |
| [docs hub](README.md) | Full index |
| [rustapi-rs-examples](https://github.com/Tuntii/rustapi-rs-examples) | Larger sample apps |
