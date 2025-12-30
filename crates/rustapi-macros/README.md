# RustAPI Macros

Internal procedural macros for the RustAPI framework.

> **Note**: This is an internal crate. You should depend on `rustapi-rs` instead.

## Features

- `#[rustapi::main]`: Async runtime entry point.
- `#[rustapi::get]`: GET handler definition.
- `#[rustapi::post]`: POST handler definition.
- `#[rustapi::put]`: PUT handler definition.
- `#[rustapi::patch]`: PATCH handler definition.
- `#[rustapi::delete]`: DELETE handler definition.
- `#[rustapi::tag]`: OpenAPI tag metadata.
- `#[rustapi::summary]`: OpenAPI summary metadata.
- `#[rustapi::description]`: OpenAPI description metadata.

## Usage

These are automatically exported via the `rustapi` prefix when using `rustapi-rs`.

```rust
use rustapi_rs::prelude::*;

#[rustapi::get("/hello")]
#[rustapi::summary("Hello Endpoint")]
async fn hello() -> &'static str {
    "Hello World"
}
```
