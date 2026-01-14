# Quickstart

> [!TIP]
> From zero to a production-ready API in 60 seconds.

## Create a New Project

Use the CLI to generate a new project. We'll call it `my-api`.

```bash
cargo rustapi new my-api
cd my-api
```

This commands sets up a complete project structure with handling, models, and tests ready to go.

## Run the Server

Start your API server:

```bash
cargo run
```

You should see output similar to:

```
INFO 🚀 Server running at http://127.0.0.1:8080
INFO 📚 API docs at http://127.0.0.1:8080/docs
```

## Test It Out

Open your browser to [http://127.0.0.1:8080/docs](http://127.0.0.1:8080/docs).

You'll see the **Swagger UI** automatically generated from your code. Try out the `/health` endpoint or create a new Item in the `Items` API.

## What Just Happened?

You just launched a high-performance, async Rust web server with:
- ✅ Automatic OpenAPI documentation
- ✅ Type-safe request validation
- ✅ Distributed tracing
- ✅ Global error handling

Welcome to RustAPI.
