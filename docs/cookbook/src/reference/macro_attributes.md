# Macro Attribute Reference

RustAPI’s attribute macros do two jobs at once:

1. they register routes and schemas at compile time, and
2. they enrich the generated OpenAPI operation metadata.

This reference focuses on the route metadata attributes most users need first:

- `#[tag(...)]`
- `#[summary(...)]`
- `#[description(...)]`
- `#[param(...)]`
- `#[errors(...)]`

> **Golden rule:** In user code, use the facade macros from `rustapi-rs`, e.g. `#[rustapi_rs::get(...)]`, not internal crates.

## Typical usage

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize, Schema)]
struct User {
    id: String,
    name: String,
}

#[rustapi_rs::get("/users/{id}")]
#[rustapi_rs::tag("Users")]
#[rustapi_rs::summary("Get user by ID")]
#[rustapi_rs::description("Returns a single user by its unique identifier.")]
#[rustapi_rs::param(id, schema = "uuid")]
#[rustapi_rs::errors(404 = "User not found", 403 = "Forbidden")]
async fn get_user(Path(_id): Path<String>) -> Result<Json<User>> {
    Ok(Json(User {
        id: "550e8400-e29b-41d4-a716-446655440000".into(),
        name: "Alice".into(),
    }))
}
```

## `#[rustapi_rs::tag("...")]`

Groups the operation under one or more OpenAPI tags.

### Syntax

```rust
#[rustapi_rs::tag("Users")]
```

### Effect

- Appends the tag value to the operation’s `tags` list.
- Useful for Swagger grouping and cookbook-style API organization.

### Example

```rust
#[rustapi_rs::get("/items")]
#[rustapi_rs::tag("Items")]
async fn list_items() -> &'static str {
    "ok"
}
```

## `#[rustapi_rs::summary("...")]`

Sets the short OpenAPI summary for the operation.

### Syntax

```rust
#[rustapi_rs::summary("List all items")]
```

### Effect

- Fills the operation summary shown in Swagger and generated specs.
- Best used as a short, action-oriented sentence.

### Example

```rust
#[rustapi_rs::get("/items")]
#[rustapi_rs::summary("List all items")]
async fn list_items() -> &'static str {
    "ok"
}
```

## `#[rustapi_rs::description("...")]`

Sets the longer description for the operation.

### Syntax

```rust
#[rustapi_rs::description("Returns all active items. Supports pagination.")]
```

### Effect

- Fills the operation description field.
- Good for behavior notes, pagination semantics, or auth requirements.

### Example

```rust
#[rustapi_rs::get("/items")]
#[rustapi_rs::description("Returns active items only. Archived items are excluded.")]
async fn list_items() -> &'static str {
    "ok"
}
```

## `#[rustapi_rs::param(...)]`

Overrides the OpenAPI schema type for a **path parameter**.

This is useful when the auto-inferred type is not the schema shape you want to expose in docs.

### Supported schema types

- `"uuid"`
- `"integer"` or `"int"`
- `"string"`
- `"boolean"` or `"bool"`
- `"number"`

### Supported forms

Form 1:

```rust
#[rustapi_rs::param(id, schema = "uuid")]
```

Form 2:

```rust
#[rustapi_rs::param(id = "uuid")]
```

### Effect

- Adds a custom path parameter schema override to the generated route metadata.
- Particularly useful for IDs that are represented as strings but should be documented with UUID semantics.

### Example

```rust
#[rustapi_rs::get("/orders/{order_id}")]
#[rustapi_rs::param(order_id, schema = "uuid")]
async fn get_order(Path(_order_id): Path<String>) -> &'static str {
    "ok"
}
```

### Notes

- This attribute is intended for **path parameters**.
- RustAPI already auto-detects path params from handler signatures; `#[param(...)]` is an override, not a requirement.

## `#[rustapi_rs::errors(...)]`

Declares additional typed error responses for OpenAPI.

### Syntax

```rust
#[rustapi_rs::errors(404 = "User not found", 403 = "Forbidden")]
```

### Effect

- Adds those responses directly to the operation’s OpenAPI response map.
- Each declared response uses the standard `ErrorSchema` under `application/json`.

### Example

```rust
#[rustapi_rs::delete("/users/{id}")]
#[rustapi_rs::errors(404 = "User not found")]
async fn delete_user(Path(_id): Path<i64>) -> Result<()> {
    Ok(())
}
```

### Multiple status codes

```rust
#[rustapi_rs::post("/users")]
#[rustapi_rs::errors(
    400 = "Invalid input",
    409 = "Email already exists",
    422 = "Validation failed"
)]
async fn create_user(Json(_body): Json<User>) -> Result<Created<User>> {
    # todo!()
}
```

## Interaction with route macros

These metadata attributes are consumed by the HTTP method macros such as:

- `#[rustapi_rs::get(...)]`
- `#[rustapi_rs::post(...)]`
- `#[rustapi_rs::put(...)]`
- `#[rustapi_rs::patch(...)]`
- `#[rustapi_rs::delete(...)]`

The route macro gathers metadata from the other attributes and turns them into builder calls such as:

- `.tag(...)`
- `.summary(...)`
- `.description(...)`
- `.param(...)`
- `.error_response(...)`

## Recommended ordering

Keep the route macro first, then place metadata attributes below it:

```rust
#[rustapi_rs::get("/users/{id}")]
#[rustapi_rs::tag("Users")]
#[rustapi_rs::summary("Get user")]
#[rustapi_rs::param(id, schema = "uuid")]
#[rustapi_rs::errors(404 = "User not found")]
async fn get_user(Path(_id): Path<String>) -> Result<&'static str> {
    Ok("ok")
}
```

That matches the style already used across the repository and keeps metadata easy to scan.

## What these macros do **not** do

- They do **not** replace `#[derive(Schema)]` for your DTOs.
- They do **not** change runtime authorization or validation behavior by themselves.
- `#[errors(...)]` enriches OpenAPI docs; your handler still needs to return the appropriate `ApiError` or equivalent response at runtime.

## Common mistakes

### Forgetting `Schema` on request/response types

The metadata attributes do not remove the need for `#[derive(Schema)]` on DTOs used in OpenAPI-aware handlers.

### Using internal crates directly

Prefer:

```rust
#[rustapi_rs::tag("Users")]
```

not imports from `rustapi-macros` or `rustapi-core` in user-facing examples.

### Assuming `#[errors(...)]` changes runtime logic

It documents the operation. Your code still needs to actually return `404`, `409`, etc.

## Related reading

- [rustapi-openapi README](../../../../crates/rustapi-openapi/README.md)
- [Error Handling recipe](../recipes/error_handling.md)
- [Custom Extractors recipe](../recipes/custom_extractors.md)