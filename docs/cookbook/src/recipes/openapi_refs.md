# OpenAPI Schemas & References

RustAPI's OpenAPI generation is built around the `RustApiSchema` trait, which is automatically implemented when you derive `Schema`. This system seamlessly handles JSON Schema 2020-12 references (`$ref`) to reduce duplication and support recursive types.

## Automatic References

When you use `#[derive(Schema)]` on a struct or enum, RustAPI generates an implementation that:
1.  Registers the type in the OpenAPI `components/schemas` section.
2.  Returns a `$ref` pointing to that component whenever the type is used in another schema.

This means you don't need to manually configure references – they just work.

```rust
use rustapi_openapi::Schema;

#[derive(Schema)]
struct Address {
    street: String,
    city: String,
}

#[derive(Schema)]
struct User {
    username: String,
    // This will generate {"$ref": "#/components/schemas/Address"}
    address: Address,
}
```

## Recursive Types

Recursive types (like a Comment that replies to another Comment) are supported automatically because the schema is registered *before* its fields are processed. However, you must use `Box<T>` or `Option<T>` for the recursive field to break the infinite size cycle in Rust.

```rust
#[derive(Schema)]
struct Comment {
    id: String,
    text: String,
    // Recursive reference works automatically
    replies: Option<Vec<Box<Comment>>>,
}
```

## Generics

Generic types are also supported. The schema name will include the concrete type parameters to ensure uniqueness.

```rust
#[derive(Schema)]
struct Page<T> {
    items: Vec<T>,
    total: u64,
}

#[derive(Schema)]
struct Product {
    name: String,
}

// Generates component: "Page_Product"
// Generates usage: {"$ref": "#/components/schemas/Page_Product"}
async fn list_products() -> Json<Page<Product>> { ... }
```

## Renaming & Customization

You can customize how fields appear in the schema using standard Serde attributes, as `rustapi-openapi` respects `#[serde(rename)]`.

```rust
#[derive(Schema, Serialize)]
struct UserConfig {
    #[serde(rename = "userId")]
    user_id: String, // In schema: "userId"
}
```

Note: Currently, `#[derive(Schema)]` does not support specific `#[schema(...)]` attributes for descriptions or examples directly on fields. You should use doc comments (if supported in future versions) or implement `RustApiSchema` manually for advanced customization.

## Manual Implementation

If you need a schema that cannot be derived (e.g., for a third-party type), you can implement `RustApiSchema` manually.

```rust
use rustapi_openapi::schema::{RustApiSchema, SchemaCtx, SchemaRef, JsonSchema2020};

struct MyCustomType;

impl RustApiSchema for MyCustomType {
    fn schema(ctx: &mut SchemaCtx) -> SchemaRef {
        let name = "MyCustomType";

        // Register if not exists
        if ctx.components.contains_key(name) {
             return SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) };
        }

        // Insert placeholder
        ctx.components.insert(name.to_string(), JsonSchema2020::new());

        // Build schema
        let mut schema = JsonSchema2020::string();
        schema.format = Some("custom-format".to_string());

        // Update component
        ctx.components.insert(name.to_string(), schema);

        SchemaRef::Ref { reference: format!("#/components/schemas/{}", name) }
    }

    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("MyCustomType")
    }
}
```
