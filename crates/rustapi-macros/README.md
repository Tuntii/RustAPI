# RustAPI Macros

**Procedural macros that power the RustAPI developer experience.**

> ℹ️ **Note**: These macros are re-exported by `rustapi-rs`. You do not need to add this crate manually.

## Attribute Macros

### `#[rustapi_rs::main]`
Replaces `#[tokio::main]`. Sets up the runtime, tracing subscriber, and other framework essentials.

### HTTP Method Handlers
Registers a function as a route handler.

- `#[rustapi_rs::get("/users/{id}")]`
- `#[rustapi_rs::post("/users")]`
- `#[rustapi_rs::put("/users/{id}")]`
- `#[rustapi_rs::delete("/users/{id}")]`
- `#[rustapi_rs::patch("/users/{id}")]`
- `#[rustapi_rs::head("/health")]`
- `#[rustapi_rs::options("/cors")]`

### OpenAPI Metadata
Enrich your auto-generated documentation.

- `#[rustapi_rs::tag("Auth")]`: Groups endpoints.
- `#[rustapi_rs::summary("Logs in a user")]`: Brief summary.
- `#[rustapi_rs::description("Full markdown description...")]`: Detailed docs.

## Derive Macros

### `#[derive(Schema)]`
Generates a JSON Schema for the struct, used by `rustapi-openapi`.
*Wraps `utoipa::ToSchema` via `rustapi-openapi` integration.*

### `#[derive(Validate)]`
Generates validation logic.
*Wraps `validator::Validate` via `rustapi-validate` integration.*
