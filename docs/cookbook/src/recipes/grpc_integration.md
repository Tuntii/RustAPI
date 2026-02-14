# gRPC Integration

RustAPI allows you to seamlessly integrate gRPC services alongside your HTTP API, running both on the same Tokio runtime or even the same port (with proper multiplexing, though separate ports are simpler). We use the `rustapi-grpc` crate, which provides helpers for [Tonic](https://github.com/hyperium/tonic).

## Dependencies

Add the following to your `Cargo.toml`:

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["grpc"] }
tonic = "0.10"
prost = "0.12"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }

[build-dependencies]
tonic-build = "0.10"
```

## Defining the Service (Proto)

Create a `proto/helloworld.proto` file:

```protobuf
syntax = "proto3";

package helloworld;

service Greeter {
  rpc SayHello (HelloRequest) returns (HelloReply);
}

message HelloRequest {
  string name = 1;
}

message HelloReply {
  string message = 1;
}
```

## The Build Script

In `build.rs`:

```rust,no_run
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/helloworld.proto")?;
    Ok(())
}
```

## Implementation

Here is how to run both servers concurrently with shared shutdown.

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_rs::grpc::{run_rustapi_and_grpc_with_shutdown, tonic};
use tonic::{Request, Response, Status};

// Import generated proto code (simplified for example)
pub mod hello_world {
    tonic::include_proto!("helloworld");
}
use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

// --- gRPC Implementation ---
#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let name = request.into_inner().name;
        let reply = hello_world::HelloReply {
            message: format!("Hello {} from gRPC!", name),
        };
        Ok(Response::new(reply))
    }
}

// --- HTTP Implementation ---
#[rustapi_rs::get("/health")]
async fn health() -> Json<&'static str> {
    Json("OK")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Define HTTP App
    let http_app = RustApi::new().route("/health", get(health));
    let http_addr = "0.0.0.0:3000";

    // 2. Define gRPC Service
    let grpc_addr = "0.0.0.0:50051".parse()?;
    let greeter = MyGreeter::default();

    println!("HTTP listening on http://{}", http_addr);
    println!("gRPC listening on grpc://{}", grpc_addr);

    // 3. Run both with shared shutdown (Ctrl+C)
    run_rustapi_and_grpc_with_shutdown(
        http_app,
        http_addr,
        tokio::signal::ctrl_c(),
        move |shutdown| {
            tonic::transport::Server::builder()
                .add_service(GreeterServer::new(greeter))
                .serve_with_shutdown(grpc_addr, shutdown)
        },
    ).await?;

    Ok(())
}
```

## How It Works

1.  **Shared Runtime**: Both servers run on the same Tokio runtime, sharing thread pool resources efficiently.
2.  **Graceful Shutdown**: When `Ctrl+C` is pressed, `run_rustapi_and_grpc_with_shutdown` signals both the HTTP server and the gRPC server to stop accepting new connections and finish pending requests.
3.  **Simplicity**: You don't need to manually spawn tasks or manage channels for shutdown signals.

## Advanced: Multiplexing

To run both HTTP and gRPC on the **same port**, you would typically use a library like `tower` to inspect the `Content-Type` header (`application/grpc` vs others) and route accordingly. However, running on separate ports (e.g., 8080 for HTTP, 50051 for gRPC) is standard practice in Kubernetes and most deployment environments.
