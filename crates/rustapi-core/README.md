# rustapi-core

**Lens**: "The Engine"  
**Philosophy**: "The foundational crate that glues everything together."

> ⚠️ **Warning**: Most users should depend on `rustapi-rs` instead of using this crate directly. This crate's API is subject to change to support the higher-level facade.

## Core Responsibilities

1. **Routing**: Mapping HTTP requests to Handlers
2. **Extraction**: The `FromRequest` trait definition
3. **Response**: The `IntoResponse` trait definition
4. **Middleware**: The `Layer` and `Service` integration with Tower
5. **HTTP/3**: Built-in QUIC support via `h3` and `quinn` (optional feature)

## The `Router` Internals

We use `matchit`, a high-performance **Radix Tree** implementation for routing.

### Why Radix Trees?
- **Speed**: Lookup time is proportional to the length of the path, not the number of routes
- **Priority**: Specific paths (`/users/profile`) always take precedence over wildcards (`/users/{id}`)
- **Parameters**: Efficiently parses named parameters like `{id}` or `*path` without regular expressions

## The `Handler` Trait Magic

The `Handler` trait is what allows you to write functions with arbitrary arguments:

```rust
// This looks simple...
async fn my_handler(state: State<Db>, json: Json<Data>) { ... }

// ...but under the hood, it compiles to something like:
impl Handler for my_handler {
    fn call(req: Request) -> Future<Output=Response> {
        // 1. Extract State
        // 2. Extract Json
        // 3. Call original function
        // 4. Convert return to Response
    }
}
```

This is achieved through **recursive trait implementations** on tuples. RustAPI supports handlers with up to **16 arguments**.

## Middleware Architecture

`rustapi-core` is built on top of `tower`. This means any standard Tower middleware works out of the box.

```rust
// The Service stack looks like an onion:
// Outer Layer (Timeout)
//  -> Middle Layer (Trace)
//      -> Inner Layer (Router)
//          -> Handler
```

When you call `.layer()`, you are wrapping the inner service with a new outer layer.
