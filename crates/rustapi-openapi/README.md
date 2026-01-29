# RustAPI OpenAPI

**Automated OpenAPI 3.0/3.1 specifications and Swagger UI integration with native schema generation.**

> ℹ️ **Note**: This crate is used internally by `rustapi-rs` to provide the `.docs()` method on the server builder.

## Features

- **Native Schema Generation**: Generate OpenAPI schemas without external dependencies
- **OpenAPI 3.0.3** and **OpenAPI 3.1.0** specification support
- **Swagger UI** embedded serving at your specified path
- **ReDoc** documentation interface (optional)
- **API Versioning** with multiple strategies (path, header, query, accept)
- **Webhook Definitions** support (OpenAPI 3.1)
- **Optional utoipa integration** for backward compatibility

## How It Works

1. **Native Traits**: Implement `ToOpenApiSchema` for your types to generate JSON Schemas natively
2. **Schema Builders**: Use fluent API builders to construct complex schemas
3. **Spec Build**: At runtime, it assembles the full OpenAPI specification
4. **UI Serve**: Embeds Swagger UI assets and serves them at your specified path

## Native Schema Generation (Recommended)

Use the native traits and builders to generate schemas without external dependencies:

```rust
use rustapi_openapi::native::{ToOpenApiSchema, NativeSchema, ObjectSchemaBuilder};
use std::borrow::Cow;
use serde_json::Value;

// Implement ToOpenApiSchema for your types
struct User {
    id: i64,
    name: String,
    email: Option<String>,
}

impl ToOpenApiSchema for User {
    fn schema() -> (Cow<'static, str>, Value) {
        (
            "User".into(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "format": "int64" },
                    "name": { "type": "string" },
                    "email": { "type": "string", "nullable": true }
                },
                "required": ["id", "name"]
            })
        )
    }
}

// Or use the builder API
let user_schema = ObjectSchemaBuilder::new()
    .title("User")
    .description("A user in the system")
    .required_integer("id")
    .required_string("name")
    .optional_string("email")
    .build();
```

## Using with OpenAPI Spec

```rust
use rustapi_openapi::{OpenApiSpec, ToOpenApiSchema};

let spec = OpenApiSpec::new("My API", "1.0.0")
    .description("My awesome API")
    .register_native::<User>()  // Register native schema types
    .schema("CustomSchema", serde_json::json!({"type": "string"}));
```

## Optional utoipa Integration

For backward compatibility, enable the `utoipa` feature to use `#[derive(Schema)]`:

```toml
[dependencies]
rustapi-openapi = { version = "0.1", features = ["utoipa"] }
```

```rust
use rustapi_openapi::Schema;

#[derive(Schema)]
struct User {
    id: i64,
    name: String,
}
```

## Features Flags

- `default` = `["swagger-ui", "utoipa"]`
- `swagger-ui` - Enable Swagger UI serving
- `redoc` - Enable ReDoc documentation UI
- `utoipa` - Enable utoipa integration for `#[derive(Schema)]`

## Customization

```rust
RustApi::new()
    .api_name("My Enterprise API")
    .api_version("2.1.0")
    .docs("/swagger-ui")
    .run("0.0.0.0:3000")
    .await
```
