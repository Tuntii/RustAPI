# Modular OpenAPI Schemas

RustAPI's OpenAPI generator is designed to produce modular, reusable schemas using JSON Schema 2020-12 references (`$ref`). This keeps your API specification clean and reduces duplication.

## Automatic Reference Generation

When you use `#[derive(Schema)]` on a struct or enum, RustAPI automatically:
1. Generates a unique name for the schema (e.g., `User`, `CreateUserRequest`).
2. Registers the full schema definition in `components/schemas`.
3. Uses a reference (`$ref: "#/components/schemas/User"`) wherever that type is used.

### Example

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize, Schema)]
struct Address {
    street: String,
    city: String,
}

#[derive(Serialize, Schema)]
struct User {
    id: i64,
    username: String,
    // This will be a reference to the Address schema
    address: Address,
}

#[derive(Serialize, Schema)]
struct Company {
    name: String,
    // Reusing the same Address schema
    hq_address: Address,
}
```

The generated OpenAPI JSON will look like this (simplified):

```json
{
  "components": {
    "schemas": {
      "Address": {
        "type": "object",
        "properties": {
          "street": { "type": "string" },
          "city": { "type": "string" }
        },
        "required": ["street", "city"]
      },
      "User": {
        "type": "object",
        "properties": {
          "id": { "type": "integer", "format": "int64" },
          "username": { "type": "string" },
          "address": { "$ref": "#/components/schemas/Address" }
        }
      },
      "Company": {
        "type": "object",
        "properties": {
          "name": { "type": "string" },
          "hq_address": { "$ref": "#/components/schemas/Address" }
        }
      }
    }
  }
}
```

## Generic Types

RustAPI handles generic types by generating unique names based on the type parameters.

```rust
#[derive(Serialize, Schema)]
struct Page<T> {
    items: Vec<T>,
    total: u64,
}

// Used as Page<User> -> Schema name: "Page_User"
// Used as Page<Company> -> Schema name: "Page_Company"
```

## Circular References

Because `derive(Schema)` registers the name *before* building the full schema (if encountered recursively), it supports recursive types naturally.

```rust
#[derive(Serialize, Schema)]
struct Category {
    name: String,
    // Recursive reference
    sub_categories: Vec<Category>,
}
```

This works because `Vec<Category>` calls `Category::schema()`, which sees that `Category` is already being visited/registered and returns a `$ref` immediately.

## Manual Registration

If you need to register a schema manually (e.g., for a type you don't control), you can implement `RustApiSchema` yourself, or use `RustApi::register_schema::<T>()` to ensure it appears in components even if not used in any route.

```rust
RustApi::new()
    .register_schema::<MyExternalType>()
    // ...
```
