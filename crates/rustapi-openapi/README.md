# rustapi-openapi

**Lens**: "The Documentarian"  
**Philosophy**: "Your API speaks for itself."

Automated API specifications and Swagger UI integration for RustAPI.

> ℹ️ **Note**: This crate is used internally by `rustapi-rs` to provide the `.docs()` method on the server builder.

## How It Works

1. **Reflection**: RustAPI macros collect metadata about your routes (path, method, input types, output types) at compile time
2. **Schema Gen**: Uses `utoipa` to generate JSON Schemas for your Rust structs
3. **Spec Build**: At runtime, assembles the full OpenAPI 3.0 JSON specification
4. **UI Serve**: Embeds the Swagger UI assets and serves them at your specified path

## Route Metadata Macros

RustAPI provides attribute macros for enriching OpenAPI documentation:

```rust
#[rustapi_rs::get("/users/{id}")]
#[rustapi_rs::tag("Users")]
#[rustapi_rs::summary("Get user by ID")]
#[rustapi_rs::description("Returns a single user by their unique identifier.")]
async fn get_user(Path(id): Path<i64>) -> Json<User> { ... }
```

## Customization

Inject custom security schemes or info into the spec via the `RustApi` builder:

```rust
RustApi::new()
    .api_name("My Enterprise API")
    .api_version("2.1.0")
    .docs("/swagger-ui")
    .run("0.0.0.0:3000")
    .await
```
