# RustAPI OpenAPI

OpenAPI documentation generation for the RustAPI framework.

> **Note**: This is an internal crate. You should depend on `rustapi-rs` instead.

## Features

- **Auto-generation**: Generates OpenAPI v3 specification from your code.
- **Swagger UI**: Serves an interactive documentation page.
- **Schema Derivation**: `#[derive(Schema)]` for structs (re-exports `utoipa::ToSchema`).
- **Standard Schemas**: Includes common schemas like `ErrorSchema`, `ValidationErrorSchema`.

## Integration

This crate is tightly integrated into `rustapi-core`.

```rust
use rustapi_openapi::{OpenApiSpec, ErrorSchema};

// Create a spec
let spec = OpenApiSpec::new("My API", "1.0")
    .register::<ErrorSchema>();
```
