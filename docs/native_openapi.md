# Native OpenAPI Design for RustAPI

> **Status**: ✅ **IMPLEMENTED** (v0.1.203)
> 
> This design document describes the native OpenAPI 3.1 generator that replaced the `utoipa` dependency.

## Overview

RustAPI has moved to a native OpenAPI 3.1 generator, eliminating external dependencies (specifically `utoipa`) and ensuring deterministic, high-quality spec generation. This document outlines the architecture of the "Native OpenAPI" implementation.

## Goals ✅

- **Zero Heavy Dependencies**: ✅ Removed `utoipa`. Uses only `serde` + `serde_json` + standard library.
- **OpenAPI 3.1**: ✅ Full JSON Schema 2020-12 compatibility.
- **Determinism**: ✅ Output is stable (sorted keys via `BTreeMap`, stable component names).
- **Backwards Compatibility**: ✅ Maintains existing `RustApi::auto()` experience.

## Architecture

### 1. Capture (Macros) ✅

Endpoint metadata is captured at compile-time using procedural macros:
- `#[rustapi::get("/path")]`: Captures method, path, handler function, and doc comments.
- `#[derive(Schema)]`: Generates `impl RustApiSchema` for data types.
- `#[derive(TypedPath)]`: Captures path parameter structure.

### 2. Collection (Registry) ✅

We utilize the existing `linkme` based distributed slice mechanism in `rustapi-core` to collect routes and schemas at runtime without manual registration.

- `AUTO_ROUTES`: A distributed slice of route factory functions.
- `AUTO_SCHEMAS`: A distributed slice of schema registration functions.

When `RustApi::auto()` is called, the runtime iterates these slices and registers them into the `OpenApiSpec`.

### 3. Generation (RustApiSchema) ✅

The `RustApiSchema` trait in `rustapi-openapi` defines how Rust types map to JSON Schema.

```rust
pub trait RustApiSchema {
    /// Generate a schema reference (inline or $ref)
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef;

    /// Get the stable component name (if applicable)
    fn component_name() -> Option<&'static str> { None }
    
    /// Get field schemas for struct types (used by Query extractor)
    fn field_schemas(ctx: &mut SchemaCtx) -> Option<BTreeMap<String, SchemaRef>> { None }
}
```

**SchemaCtx**:
- Manages the `components/schemas` map.
- Handles deduplication: If a type has a component name, it checks if it's already registered. If not, it generates the schema and stores it.
- Returns `$ref: "#/components/schemas/Name"` for registered components.

**Strategies**:
- **Primitives**: Inline standard JSON schemas (string, integer, etc.) with format annotations.
- **Option<T>**: OpenAPI 3.1 style `type: ["string", "null"]` or `oneOf` for complex types.
- **Vec<T>**: Array schema with `items: schema(T)`.
- **HashMap<String, T>**: Object with `additionalProperties: schema(T)`.
- **Enums**:
  - String enums → `enum: ["A", "B"]`
  - Tagged unions → `oneOf` with `discriminator`.

### 4. OpenAPI Model ✅

The `rustapi-openapi` crate defines the OpenAPI 3.1 model using strictly `BTreeMap` to ensure output stability.

```rust
pub struct OpenApiSpec {
    pub openapi: String, // "3.1.0"
    pub info: ApiInfo,
    pub json_schema_dialect: Option<String>, // "https://json-schema.org/draft/2020-12/schema"
    pub servers: Vec<Server>,
    pub paths: BTreeMap<String, PathItem>,
    pub webhooks: BTreeMap<String, PathItem>,
    pub components: Option<Components>,
    pub security: Vec<BTreeMap<String, Vec<String>>>,
    pub tags: Vec<Tag>,
    pub external_docs: Option<ExternalDocs>,
}
```

### 5. Runtime Integration ✅

- **Handler Trait**: Updated to use `RustApiSchema` for request/response body definition.
- **Wrappers**: `Json<T>`, `Path<T>`, `Query<T>` implement schema generation by delegating to `T`.
  - `Json<T>` → content: application/json → schema: T
  - `Path<T>` → parameters (in: path)
  - `Query<T>` → parameters (in: query) via `field_schemas()`

### 6. Serving ✅

- `/openapi.json`: Serializes the `OpenApiSpec` to pretty JSON.
- `/docs`: Serves a minimal HTML page that loads Swagger UI from CDN (unpkg), pointing to `/openapi.json`.

## Migration from utoipa

```rust
// Before (v0.1.202 and earlier)
use utoipa::ToSchema;

#[derive(ToSchema)]
struct CreateUser {
    name: String,
    email: String,
}

// After (v0.1.203+)
use rustapi_rs::prelude::*;

#[derive(Schema)]
struct CreateUser {
    name: String,
    email: String,
}
```

The `Schema` derive macro now generates `impl RustApiSchema` instead of `impl ToSchema`. All extractors (`Json<T>`, `ValidatedJson<T>`, `Query<T>`) have been updated to use the new trait bound.

## Benefits

1. **~500 fewer dependencies**: Removing `utoipa` significantly reduces compile times and binary size.
2. **OpenAPI 3.1 native**: Full support for JSON Schema 2020-12 features like `type: ["string", "null"]`.
3. **Deterministic output**: `BTreeMap` ensures consistent JSON ordering for diffing and caching.
4. **Compile-time generation**: Schemas are generated at compile-time, reducing runtime overhead.
