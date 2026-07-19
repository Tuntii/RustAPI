# Quickstart

> Prefer the repo **[Golden Path](../../GOLDEN_PATH.md)** for the full story (OpenAPI path, probes, MCP/deploy forks).

This page is the CLI-generated project path.

## Install the CLI

```bash
cargo install cargo-rustapi
```

## Create a New Project

```bash
cargo rustapi new my-api
cd my-api
```

> If `cargo rustapi` does not work, run `cargo-rustapi new my-api` directly.

## The Code

`src/main.rs` should look like:

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/hello")]
async fn hello() -> Json<String> {
    Json("Hello from RustAPI!".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    RustApi::auto()
        .production_defaults("my-api")
        .run("127.0.0.1:8080")
        .await
}
```

For the minimal macro + probes example without the CLI, clone this monorepo and run:

```bash
cargo run -p rustapi-rs --example golden_path
```

## Run the Server

```bash
cargo run
```

## Test It Out

- App: `curl http://127.0.0.1:8080/hello` (or `/api/v1/ping` on golden_path)
- Docs: [http://127.0.0.1:8080/docs](http://127.0.0.1:8080/docs)
- Spec: `http://127.0.0.1:8080/docs/openapi.json`

## What Just Happened?

- Automatic OpenAPI from route macros + `Schema`
- Swagger UI at `/docs`
- With `production_defaults`: request IDs, tracing, `/live` `/ready` `/health`

Next: [Golden Path](../../GOLDEN_PATH.md) · [Installation](installation.md) · [Structure](structure.md)
