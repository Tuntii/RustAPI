# Quickstart

> [!TIP]
> From zero to a production-ready API in 60 seconds.

## Install the CLI

First, install the RustAPI CLI tool:

```bash
cargo install cargo-rustapi
```

## Create a New Project

Use the CLI to generate a new project. We'll call it `my-api`.

```bash
cargo rustapi new my-api
cd my-api
```

> **Note**: If `cargo rustapi` doesn't work, you can also run `cargo-rustapi new my-api` directly.

This command sets up a complete project structure with handling, models, and tests ready to go.

## The Code

Open `src/main.rs`. You'll see how simple it is:

```rust
use rustapi_rs::prelude::*;

#[rustapi::get("/hello")]
async fn hello() -> Json<String> {
    Json("Hello from RustAPI!".to_string())
}

#[rustapi::main]
async fn main() -> Result<()> {
    // Auto-discovery magic ✨
    RustApi::auto()
        .run("127.0.0.1:8080")
        .await
}
```

## Run the Server

Start your API server:

```bash
cargo run
```

You should see output similar to:

```
INFO rustapi: 🚀 Server running at http://127.0.0.1:8080
INFO rustapi: 📚 API docs at http://127.0.0.1:8080/docs
```

## Test It Out

Open your browser to [http://127.0.0.1:8080/docs](http://127.0.0.1:8080/docs).

You'll see the **Swagger UI** automatically generated from your code. Try out the endpoint directly from the browser!

## What Just Happened?

You just launched a high-performance, async Rust web server with:
- ✅ Automatic OpenAPI documentation
- ✅ Type-safe request validation
- ✅ Distributed tracing
- ✅ Global error handling

Welcome to RustAPI.
