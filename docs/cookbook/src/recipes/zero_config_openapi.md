# Zero-Config OpenAPI

**Problem**: You want to document your API automatically without writing separate YAML files or complex builder code.
**Solution**: Use RustAPI's native attribute macros and auto-discovery.

## The "Native" Approach

Instead of manually mounting routes and defining operations, RustAPI allows you to declare routes directly on your handler functions using attributes.

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::{IntoParams, ToSchema};

#[derive(Serialize, Deserialize, ToSchema)]
struct User {
    id: i32,
    username: String,
}

#[derive(Deserialize, IntoParams)]
struct SearchParams {
    q: String,
    page: Option<usize>,
}

// 1. Decorate your handlers
#[rustapi::get("/users")]
async fn list_users(Query(params): Query<SearchParams>) -> Json<Vec<User>> {
    // ...
}

#[rustapi::post("/users")]
async fn create_user(Json(user): Json<User>) -> Created<User> {
    Created(user)
}

#[tokio::main]
async fn main() -> Result<()> {
    // 2. Use RustApi::auto() to automatically find and register all decorated routes
    RustApi::auto()
        .run("127.0.0.1:8080")
        .await
}
```

## How It Works

1.  **Macros**: The `#[rustapi::get]`, `#[rustapi::post]`, etc., macros generate a distributed inventory of routes at compile time.
2.  **Auto-Discovery**: `RustApi::auto()` collects these inventory items.
3.  **Schema Inference**:
    *   **Request Body**: Inferred from `Json<T>` arguments (requires `T: ToSchema`).
    *   **Query Params**: Inferred from `Query<T>` arguments (requires `T: IntoParams`).
    *   **Path Params**: Inferred from `Path<T>` and the URL path (e.g., `/users/{id}`).
    *   **Responses**: Inferred from the return type.

## Advanced Usage

### Customizing Metadata

You can override or enhance the generated OpenAPI spec using specific attributes:

```rust
#[rustapi::get("/items/{id}")]
#[rustapi::tag("Inventory")]
#[rustapi::summary("Find a specific item")]
#[rustapi::description("Detailed description supported here.")]
#[rustapi::response(404, description = "Item not found")]
async fn get_item(Path(id): Path<i32>) -> Result<Json<Item>> {
    // ...
}
```

### Path Parameter Types

RustAPI tries to guess types from variable names (e.g., `id` -> integer), but you can be explicit:

```rust
#[rustapi::get("/users/{uuid}")]
#[rustapi::param(uuid, schema = "uuid")] // Force UUID format
async fn get_user(Path(uuid): Path<Uuid>) -> Json<User> {
    // ...
}
```

## Discussion

This approach (often called "Code First") keeps your documentation in sync with your implementation. If you change a struct field, the documentation updates automatically. If you remove a handler, the endpoint disappears from the docs.

The `RustApi::auto()` function is the key enabler here. It scans the binary for the inventory records created by the macros. This means you don't even need to `mod` or `use` your handler modules in `main.rs` if they are in the same crate! (Though in Rust, modules usually need to be reachable to be compiled).
