# Native OpenAPI Design for RustAPI

## Overview
RustAPI uses a native OpenAPI 3.1 generator to reduce external dependencies and ensure deterministic, high-quality spec generation. This document outlines the architecture for the "Native OpenAPI" implementation.

## Goals
- **Zero Heavy Dependencies**: Use only `serde` + `serde_json` + standard library.
- **OpenAPI 3.1**: Support full JSON Schema 2020-12 compatibility.
- **Determinism**: Output MUST be stable (sorted keys, stable component names).
- **Backwards Compatibility**: Maintain existing `RustApi::auto()` experience.

## Architecture

### 1. Capture (Macros)
Endpoint metadata is captured at compile-time using procedural macros:
- `#[rustapi_rs::get("/path")]`: Captures method, path, handler function, and doc comments.
- `#[derive(Schema)]`: Generates `impl RustApiSchema` for data types.
- `#[derive(TypedPath)]`: Captures path parameter structure.

### 2. Collection (Registry)
We utilize the existing `linkme` based distributed slice mechanism in `rustapi-core` to collect routes and schemas at runtime without manual registration.

- `AUTO_ROUTES`: A distributed slice of route factory functions.
- `AUTO_SCHEMAS`: A distributed slice of schema registration functions.

When `RustApi::auto()` is called, the runtime iterates these slices and registers them into the `OpenApiSpec`.

### 3. Generation (RustApiSchema)
A new trait `RustApiSchema` in `rustapi-openapi` defines how Rust types map to JSON Schema.

```rust
pub trait RustApiSchema {
    /// Generate a schema reference (inline or $ref)
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef;

    /// Get the stable component name (if applicable)
    fn component_name() -> Option<&'static str> { None }
}
```

**SchemaCtx**:
- Manages the `components/schemas` map.
- Handles deduplication: If a type has a component name, it checks if it's already registered. If not, it generates the schema and stores it.
- Returns `$ref: "#/components/schemas/Name"` for registered components.

**Strategies**:
- **Primitives**: Inline standard JSON schemas (string, integer, etc.).
- **Option<T>**: OpenAPI 3.1 style `type: ["string", "null"]` or `oneOf` for complex types.
- **Vec<T>**: Array schema with `items: schema(T)`.
- **Enums**:
  - String enums -> `enum: ["A", "B"]`
  - Tagged unions -> `oneOf` with `discriminator`.

### 4. OpenAPI Model
The `rustapi-openapi` crate defines the OpenAPI 3.1 model using strictly `BTreeMap` to ensure output stability.

```rust
pub struct OpenApiSpec {
    pub openapi: String, // "3.1.0"
    pub info: Info,
    pub paths: BTreeMap<String, PathItem>,
    pub components: Components,
    // ...
}
```

### 5. Runtime Integration
- **Handler Trait**: Updated to use `RustApiSchema` for request/response body definition.
- **Wrappers**: `Json<T>`, `Path<T>`, `Query<T>` implement schema generation by delegating to `T`.
  - `Json<T>` -> content: application/json -> schema: T
  - `Path<T>` -> parameters (in: path)
  - `Query<T>` -> parameters (in: query)

### 6. Serving
- `/openapi.json`: Serializes the `OpenApiSpec` to pretty JSON.
- `/docs`: Serves a minimal HTML page that loads Swagger UI from a CDN (unpkg/cdnjs), pointing to the local `/openapi.json`.
