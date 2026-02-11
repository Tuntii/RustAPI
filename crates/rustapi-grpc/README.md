# rustapi-grpc

`rustapi-grpc` provides gRPC integration helpers for RustAPI with [Tonic](https://github.com/hyperium/tonic).

## What it gives you

- `run_concurrently(http, grpc)`: run two server futures together.
- `run_rustapi_and_grpc(app, http_addr, grpc)`: convenience helper for RustAPI + gRPC side-by-side.
- `run_rustapi_and_grpc_with_shutdown(app, http_addr, signal, grpc_with_shutdown)`: shared shutdown signal for both servers.
- Re-exports: `tonic`, `prost`.

## Usage

### Via `rustapi-rs` (recommended)

Add to your `Cargo.toml`:

```toml
[dependencies]
rustapi-rs = { version = "0.1", features = ["grpc"] }
```

Then import via the `grpc` module:

```rust,ignore
use rustapi_rs::grpc::{run_rustapi_and_grpc, tonic};
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/health")]
async fn health() -> &'static str { "ok" }
```

### Direct `rustapi-grpc` usage

Add to your `Cargo.toml`:

```toml
[dependencies]
rustapi-grpc = "0.1"
rustapi-core = "0.1"
```

Then import directly:

```rust,ignore
use rustapi_grpc::{run_rustapi_and_grpc, tonic};
use rustapi_core::{get, RustApi};

#[rustapi_core::get("/health")]
async fn health() -> &'static str { "ok" }
```

## Example

```rust,ignore
use rustapi_grpc::{run_rustapi_and_grpc, tonic};
use rustapi_core::{get, RustApi};

#[rustapi_core::get("/health")]
async fn health() -> &'static str { "ok" }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let http_app = RustApi::new().route("/health", get(health));

    let grpc_addr = "127.0.0.1:50051".parse()?;
    let grpc_server = tonic::transport::Server::builder()
        .add_service(MyGreeterServer::new(MyGreeter::default()))
        .serve(grpc_addr);

    run_rustapi_and_grpc(http_app, "127.0.0.1:8080", grpc_server).await?;
    Ok(())
}
```

## Shared shutdown (Ctrl+C)

```rust,ignore
use rustapi_grpc::{run_rustapi_and_grpc_with_shutdown, tonic};

let grpc_addr = "127.0.0.1:50051".parse()?;

run_rustapi_and_grpc_with_shutdown(
    http_app,
    "127.0.0.1:8080",
    tokio::signal::ctrl_c(),
    move |shutdown| {
        tonic::transport::Server::builder()
            .add_service(MyGreeterServer::new(MyGreeter::default()))
            .serve_with_shutdown(grpc_addr, shutdown)
    },
).await?;
```
