# Migrating to Native OpenAPI (v0.1.203)

This guide helps you migrate from the `utoipa`-based OpenAPI generation to RustAPI's native OpenAPI 3.1 implementation.

## Overview

In v0.1.203, RustAPI replaced the `utoipa` dependency with a native OpenAPI 3.1 generator. This change:

- **Removes ~500 transitive dependencies**
- **Upgrades to OpenAPI 3.1.0** (from 3.0.3)
- **Uses JSON Schema 2020-12** (from Draft-07)
- **Provides deterministic output** via `BTreeMap`

## Quick Migration

### Step 1: Update Cargo.toml

Remove any direct `utoipa` dependency if you had one:

```toml
# Remove this:
# utoipa = "4"

# Keep this:
[dependencies]
rustapi-rs = "0.1.203"
```

### Step 2: Update Imports

```rust
// Before
use utoipa::ToSchema;
use utoipa::IntoParams;

// After
use rustapi_rs::prelude::*;
// Schema and IntoParams are now part of the prelude
```

### Step 3: Update Derive Macros

```rust
// Before
#[derive(ToSchema)]
struct User {
    id: i64,
    name: String,
}

// After
#[derive(Schema)]
struct User {
    id: i64,
    name: String,
}
```

### Step 4: Update Query Parameter Types

If you used `IntoParams` for query parameters:

```rust
// Before
#[derive(IntoParams)]
struct Pagination {
    page: Option<i32>,
    limit: Option<i32>,
}

// After - Same derive, now generates RustApiSchema
#[derive(Schema)]
struct Pagination {
    page: Option<i32>,
    limit: Option<i32>,
}
```

## Attribute Changes

### Schema Attributes

Some `utoipa` schema attributes may need adjustment:

```rust
// Before (utoipa)
#[derive(ToSchema)]
struct Metric {
    #[schema(minimum = 0, maximum = 100)]
    value: i32,
}

// After (native) - Validation attributes go on validators
#[derive(Schema, Validate)]
struct Metric {
    #[validate(range(min = 0, max = 100))]
    value: i32,
}
```

> **Note**: The native `#[derive(Schema)]` focuses on type structure. Use `rustapi-validate` for value constraints.

### Enum Schemas

Enums work the same way:

```rust
#[derive(Schema)]
enum Status {
    Active,
    Inactive,
    Pending,
}
```

This generates: `{ "type": "string", "enum": ["Active", "Inactive", "Pending"] }`

## API Changes

### OpenApiSpec Builder

```rust
// Before
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(get_users), components(schemas(User)))]
struct ApiDoc;

// After - Programmatic registration
let spec = OpenApiSpec::new("My API", "1.0.0")
    .register::<User>()
    .description("My awesome API");
```

### Manual Schema Registration

```rust
// Before
spec.register_schema::<User>();

// After
spec.register::<User>();
// or in-place:
spec.register_in_place::<User>();
```

## Output Differences

### Nullable Types

```rust
struct Example {
    value: Option<String>,
}
```

**Before (OpenAPI 3.0.3)**:
```json
{
  "value": {
    "type": "string",
    "nullable": true
  }
}
```

**After (OpenAPI 3.1.0)**:
```json
{
  "value": {
    "type": ["string", "null"]
  }
}
```

### Version Header

**Before**: `"openapi": "3.0.3"`
**After**: `"openapi": "3.1.0"`

### JSON Schema Dialect

The new output includes:
```json
{
  "jsonSchemaDialect": "https://json-schema.org/draft/2020-12/schema"
}
```

## Swagger UI

Swagger UI is now loaded from CDN instead of bundled assets:

```rust
// Same API, different implementation
RustApi::new()
    .docs("/docs")  // Still works!
    // ...
```

The HTML now loads from `unpkg.com/swagger-ui-dist`, reducing binary size by ~2MB.

## Troubleshooting

### "trait `RustApiSchema` is not implemented"

Make sure you added `#[derive(Schema)]`:

```rust
use rustapi_rs::prelude::*;

#[derive(Schema)]  // Required!
struct MyType { ... }
```

### Missing Primitive Implementations

All standard primitives are supported:
- `i8`, `i16`, `i32`, `i64`, `i128`, `isize`
- `u8`, `u16`, `u32`, `u64`, `u128`, `usize`
- `f32`, `f64`
- `bool`, `String`, `&str`

### Nested Types

For nested custom types, ensure all types implement `RustApiSchema`:

```rust
#[derive(Schema)]
struct Inner {
    value: String,
}

#[derive(Schema)]
struct Outer {
    inner: Inner,  // Inner must also derive Schema
}
```

## Benefits of Migration

1. **Faster compilation**: ~500 fewer crates to compile
2. **Smaller binaries**: No bundled Swagger UI assets
3. **Modern spec**: OpenAPI 3.1 with JSON Schema 2020-12
4. **Deterministic output**: Consistent JSON for caching/diffing
5. **Better nullable handling**: Native `type: ["T", "null"]` syntax
