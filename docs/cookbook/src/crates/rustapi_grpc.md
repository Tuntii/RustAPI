# rustapi-grpc: The Bridge

**Lens**: "The Bridge"  
**Philosophy**: "HTTP and gRPC, one runtime."

`rustapi-grpc` is an optional crate that helps you run a RustAPI HTTP server and a Tonic gRPC server in the same process.

## What You Get

- `run_concurrently(http, grpc)` for running two server futures side-by-side.
- `run_rustapi_and_grpc(app, http_addr, grpc)` convenience helper.
- `run_rustapi_and_grpc_with_shutdown(app, http_addr, signal, grpc_with_shutdown)` for graceful shared shutdown.
- Re-exports of `tonic` and `prost`.

## Enable It

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["grpc"] }
```

## Basic Usage

```rust,ignore
use rustapi_rs::grpc::{run_rustapi_and_grpc, tonic};
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/health")]
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

## Graceful Shutdown

```rust,ignore
use rustapi_rs::grpc::{run_rustapi_and_grpc_with_shutdown, tonic};

run_rustapi_and_grpc_with_shutdown(
    http_app,
    "127.0.0.1:8080",
    tokio::signal::ctrl_c(),
    move |shutdown| {
        tonic::transport::Server::builder()
            .add_service(MyGreeterServer::new(MyGreeter::default()))
            .serve_with_shutdown("127.0.0.1:50051".parse().unwrap(), shutdown)
    },
).await?;
```
