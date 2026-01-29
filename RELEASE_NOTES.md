# RustAPI 0.2.0 Release Notes

**"Zero-Config & Native Attributes"**

This release marks a significant milestone in RustAPI's ergonomics, introducing a fully declarative "Code-First" approach to building APIs. It bridges the gap between Rust's performance and the ease of use found in frameworks like FastAPI.

## 🚀 Key Features

### 1. Zero-Config Routing (`RustApi::auto()`)
Gone are the days of manually mounting every single route in your `main` function. With the new auto-discovery mechanism, RustAPI scans your code for decorated handlers and registers them automatically.

**Before:**
```rust
// main.rs
let app = RustApi::new()
    .route("/users", get(list_users))
    .route("/users", post(create_user))
    .route("/users/{id}", get(get_user));
```

**After:**
```rust
// handlers.rs
#[rustapi::get("/users")]
async fn list_users() { ... }

// main.rs
let app = RustApi::auto(); // That's it!
```

### 2. Native OpenAPI Attributes
Define your API structure and documentation right where your code lives. The new attribute macros allow you to control every aspect of the OpenAPI spec without leaving your handler function.

```rust
#[rustapi::get("/items/{id}")]
#[rustapi::tag("Inventory")]
#[rustapi::summary("Find item by ID")]
#[rustapi::response(404, description = "Item not found")]
async fn get_item(Path(id): Path<i32>) -> Result<Json<Item>> { ... }
```

### 3. Smart Parameter Inference
RustAPI now intelligently guesses the OpenAPI data types for your path parameters based on their names, reducing the need for manual annotation.

*   `id`, `user_id` -> Inferred as `integer (int64)`
*   `uuid`, `transaction_uuid` -> Inferred as `string (uuid)`
*   Others -> Inferred as `string`

You can still override this manually if needed:
```rust
#[rustapi::param(custom_id, schema = "string")]
```

## 🛠️ Improvements & Fixes

*   **Cookbook Update**: Added a comprehensive "Zero-Config OpenAPI" recipe to the documentation.
*   **Clippy Fixes**: Resolved `clippy::to_string_trait_impl` warnings in `rustapi-openapi` for cleaner compilations.
*   **Example Updates**: `openapi_demo` example updated to showcase the new declarative style.
*   **Error Handling**: Fixed `unused_must_use` warnings in examples by properly propagating `Result` in `main`.

## 📦 Migration Guide

This release is backwards compatible. Your existing manual `.route()` calls will continue to work. To adopt the new features:
1.  Add `#[rustapi::get/post/...]` attributes to your handler functions.
2.  Switch from `RustApi::new()` to `RustApi::auto()` in your entry point.
3.  Ensure your data structs derive `ToSchema` (for bodies) and `IntoParams` (for query strings).
