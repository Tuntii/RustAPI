# rustapi-openapi: The Cartographer

**Lens**: "The Cartographer"
**Philosophy**: "Documentation as Code."

## Automatic Spec Generation
 
We believe that if documentation is manual, it is wrong. RustAPI uses `utoipa` to generate an OpenAPI 3.0 specification directly from your code.

> [!TIP]
> See the [Zero-Config OpenAPI](../recipes/zero_config_openapi.md) recipe for the modern, macro-based approach.

## The `Schema` Trait

Any type that is part of your API (request or response) must implement `Schema`.

```rust
#[derive(Schema)]
struct Metric {
    /// The name of the metric
    name: String,
    
    /// Value (0-100)
    #[schema(minimum = 0, maximum = 100)]
    value: i32,
}
```

## Operation Metadata

Use macros to enrich endpoints:

```rust
#[rustapi::get("/metrics")]
#[rustapi::tag("Metrics")]
#[rustapi::summary("List all metrics")]
#[rustapi::response(200, Json<Vec<Metric>>)]
async fn list_metrics() -> Json<Vec<Metric>> { ... }
```

## Swagger UI

The `RustApi` builder automatically mounts a Swagger UI at the path you specify:

```rust
RustApi::new()
    .docs("/docs") // Mounts Swagger UI at /docs
    // ...
```

## Path Parameter Schema Types

By default, RustAPI infers the OpenAPI schema type for path parameters based on naming conventions:
- Parameters named `id`, `user_id`, `postId`, etc. → `integer`
- Parameters named `uuid`, `user_uuid`, etc. → `string` with `uuid` format
- Other parameters → `string`

However, sometimes auto-inference is incorrect. For example, you might have a parameter named `id` that is actually a UUID. Use the `#[rustapi::param]` attribute to override the inferred type:

```rust
use uuid::Uuid;

#[rustapi::get("/users/{id}")]
#[rustapi::param(id, schema = "uuid")]
#[rustapi::tag("Users")]
async fn get_user(Path(id): Path<Uuid>) -> Json<User> {
    // The OpenAPI spec will now correctly show:
    // { "type": "string", "format": "uuid" }
    // instead of the default { "type": "integer", "format": "int64" }
    get_user_by_id(id).await
}
```

### Supported Schema Types

| Schema Type | OpenAPI Schema |
|-------------|----------------|
| `"uuid"` | `{ "type": "string", "format": "uuid" }` |
| `"integer"`, `"int"`, `"int64"` | `{ "type": "integer", "format": "int64" }` |
| `"int32"` | `{ "type": "integer", "format": "int32" }` |
| `"string"` | `{ "type": "string" }` |
| `"number"`, `"float"` | `{ "type": "number" }` |
| `"boolean"`, `"bool"` | `{ "type": "boolean" }` |

### Alternative Syntax

You can also use a shorter syntax:

```rust
// Shorter syntax: param_name = "schema_type"
#[rustapi::get("/posts/{post_id}")]
#[rustapi::param(post_id = "uuid")]
async fn get_post(Path(post_id): Path<Uuid>) -> Json<Post> { ... }
```

### Programmatic API

When building routes programmatically, you can use the `.param()` method:

```rust
use rustapi_rs::handler::get_route;

// Using the Route builder
let route = get_route("/items/{id}", get_item)
    .param("id", "uuid")
    .tag("Items")
    .summary("Get item by UUID");

app.mount_route(route);
```

