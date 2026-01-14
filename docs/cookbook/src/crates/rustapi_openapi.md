# rustapi-openapi: The Cartographer

**Lens**: "The Cartographer"
**Philosophy**: "Documentation as Code."

## Automatic Spec Generation

We believe that if documentation is manual, it is wrong. RustAPI uses `utoipa` to generate an OpenAPI 3.0 specification directly from your code.

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
