# rustapi-openapi: The Cartographer

**Lens**: "The Cartographer"
**Philosophy**: "Documentation as Code."

> **v0.1.203 Update**: RustAPI now uses a **native OpenAPI 3.1 generator**, removing the `utoipa` dependency entirely. This provides faster compile times, smaller binaries, and full JSON Schema 2020-12 support.

## Automatic Spec Generation

We believe that if documentation is manual, it is wrong. RustAPI generates an **OpenAPI 3.1.0** specification directly from your code at compile-time using the `RustApiSchema` trait.

## The `RustApiSchema` Trait

Any type that is part of your API (request or response) must implement `RustApiSchema`. Use the `#[derive(Schema)]` macro:

```rust
use rustapi_rs::prelude::*;

#[derive(Schema)]
struct Metric {
    /// The name of the metric
    name: String,
    
    /// Value (0-100)
    value: i32,
    
    /// Optional description
    description: Option<String>,
}
```

The derive macro generates:
- `impl RustApiSchema for Metric` with proper schema generation
- Automatic `$ref` to `#/components/schemas/Metric`
- Field optionality detection (`Option<T>` fields are non-required)
- Recursive schema generation for nested types

### Supported Types

| Rust Type | JSON Schema Output |
|-----------|-------------------|
| `String`, `&str` | `{ "type": "string" }` |
| `i32` | `{ "type": "integer", "format": "int32" }` |
| `i64` | `{ "type": "integer", "format": "int64" }` |
| `f32` | `{ "type": "number", "format": "float" }` |
| `f64` | `{ "type": "number", "format": "double" }` |
| `bool` | `{ "type": "boolean" }` |
| `Vec<T>` | `{ "type": "array", "items": <T schema> }` |
| `Option<T>` | `{ "type": ["<T type>", "null"] }` (OpenAPI 3.1 native) |
| `HashMap<String, T>` | `{ "type": "object", "additionalProperties": <T schema> }` |

## Operation Metadata

Use macros to enrich endpoints:

```rust
#[rustapi::get("/metrics")]
#[rustapi::tag("Metrics")]
#[rustapi::summary("List all metrics")]
async fn list_metrics() -> Json<Vec<Metric>> { ... }
```

## Swagger UI

The `RustApi` builder automatically mounts a Swagger UI at the path you specify:

```rust
RustApi::new()
    .docs("/docs") // Mounts Swagger UI at /docs (served from CDN)
    // ...
```

> **Note**: Swagger UI assets are now loaded from unpkg CDN instead of being bundled, significantly reducing binary size.

## Manual Schema Registration

For advanced use cases, you can manually register schemas:

```rust
let spec = OpenApiSpec::new("My API", "1.0.0")
    .register::<User>()        // Registers User schema
    .register::<CreateUser>()  // Registers CreateUser schema
    .description("My awesome API");
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

## Migration from v0.1.202

If upgrading from an earlier version that used `utoipa`:

```rust
// Before (utoipa-based)
use utoipa::ToSchema;
#[derive(ToSchema)]
struct User { ... }

// After (native RustApiSchema)
use rustapi_rs::prelude::*;
#[derive(Schema)]  // Generates RustApiSchema impl
struct User { ... }
```

Key changes:
- Remove `utoipa` from your `Cargo.toml`
- Replace `use utoipa::ToSchema` with `use rustapi_rs::prelude::*`
- `#[derive(ToSchema)]` becomes `#[derive(Schema)]`
- OpenAPI output is now 3.1.0 (was 3.0.3)

