# Troubleshooting: Common Gotchas

This guide covers frequently encountered issues that can be confusing when working with RustAPI. If you're stuck on a cryptic error, chances are the solution is here.

---

## 1. Missing `Schema` Derive on Extractor Types

**Symptom:**
```
error[E0277]: the trait bound `...: Handler<_>` is not satisfied
```

**Problem:**
```rust
#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub page: Option<u32>,
}
```

**Solution:**
Add the `Schema` derive macro to any struct used with extractors (`Query<T>`, `Path<T>`, `Json<T>`):

```rust
#[derive(Debug, Deserialize, Schema)]  // ✅ Schema added
pub struct ListParams {
    pub page: Option<u32>,
}
```

**Why?**
- RustAPI generates OpenAPI documentation automatically
- All extractors require `T: RustApiSchema` trait bound
- The `Schema` derive macro implements this trait for you

---

## 2. Don't Use `utoipa` Directly

**Wrong:**
```toml
[dependencies]
utoipa = "4.2"  # ❌ Don't add this
```

**Correct:**
```toml
[dependencies]
rustapi-rs = { version = "0.1.300", features = ["full"] }
# rustapi-openapi is re-exported through rustapi-rs
```

**Why?**
- RustAPI has its own OpenAPI implementation (`rustapi-openapi`)
- Adding `utoipa` directly can cause dependency conflicts
- The `Schema` derive macro is already in `rustapi_rs::prelude::*`

---

## 3. Use `rustapi_rs`, Not Internal Crates

**Symptom:**
```
error[E0432]: unresolved import `rustapi_extras`
error[E0433]: failed to resolve: use of unresolved module `rustapi_core`
error[E0433]: failed to resolve: use of unresolved module `rustapi_macros`
```

**Problem:**
```rust
use rustapi_extras::SqlxErrorExt;  // ❌ Old module name
use rustapi_core::RustApi;         // ❌ Internal crate
use rustapi_macros::get;           // ❌ Internal crate
```

**Solution:**
```rust
use rustapi_rs::prelude::*;        // ✅ Everything you need
use rustapi_rs::SqlxErrorExt;      // ✅ Correct path for extras
```

**For macros:**
```rust
// ❌ Wrong (doesn't work)
#[rustapi_macros::get("/")]
async fn index() -> &'static str { "Hello" }

// ✅ Correct
#[rustapi_rs::get("/")]
async fn index() -> &'static str { "Hello" }
```

**Why?**
- `rustapi_core`, `rustapi_macros`, `rustapi_extras` are internal implementation crates
- All public APIs are re-exported through the `rustapi-rs` facade crate
- This follows the **Facade Architecture** pattern for API stability

---

## 4. Don't Use `IntoParams` or `#[param(...)]`

**Wrong:**
```rust
#[derive(Debug, Deserialize, IntoParams)]  // ❌ IntoParams is from utoipa
pub struct ListParams {
    #[param(minimum = 1)]  // ❌ This attribute doesn't exist
    pub page: Option<u32>,
}
```

**Correct:**
```rust
#[derive(Debug, Deserialize, Schema)]  // ✅ Use Schema
pub struct ListParams {
    /// Page number (1-indexed)  // ✅ Doc comments become OpenAPI descriptions
    pub page: Option<u32>,
}
```

**For validation, use RustAPI's built-in system:**
```rust
use rustapi_rs::prelude::*;

#[derive(Debug, Deserialize, Validate, Schema)]
pub struct CreateTask {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(email)]
    pub email: String,
}

// Use ValidatedJson for automatic validation
async fn create_task(
    ValidatedJson(task): ValidatedJson<CreateTask>
) -> Result<Json<Task>> {
    // Validation runs automatically, returns 422 on failure
    Ok(Json(task))
}
```

---

## 5. `serde_json::Value` Has No Schema

**Symptom:**
```
error: the trait `RustApiSchema` is not implemented for `serde_json::Value`
```

**Problem:**
```rust
async fn handler() -> Json<serde_json::Value> {  // ❌ No schema
    Json(json!({ "key": "value" }))
}
```

**Solution - Use a typed struct (recommended):**
```rust
#[derive(Serialize, Schema)]
struct MyResponse {
    key: String,
}

async fn handler() -> Json<MyResponse> {  // ✅ Type-safe
    Json(MyResponse {
        key: "value".to_string(),
    })
}
```

**Why?**
- `serde_json::Value` doesn't implement `RustApiSchema`
- OpenAPI spec requires concrete types for documentation
- Type-safe structs catch errors at compile time

---

## 6. `DateTime<Utc>` Has No Schema

**Symptom:**
```
error[E0277]: the trait bound `DateTime<Utc>: RustApiSchema` is not satisfied
```

**Problem:**
```rust
#[derive(Debug, Serialize, Schema)]
pub struct BookmarkResponse {
    pub id: u64,
    pub created_at: DateTime<Utc>,  // ❌ No RustApiSchema impl
}
```

**Solution - Use String with RFC3339 format:**
```rust
#[derive(Debug, Serialize, Schema)]
pub struct BookmarkResponse {
    pub id: u64,
    pub created_at: String,  // ✅ Use String
}

impl From<&Bookmark> for BookmarkResponse {
    fn from(b: &Bookmark) -> Self {
        Self {
            id: b.id,
            created_at: b.created_at.to_rfc3339(),  // DateTime -> String
        }
    }
}
```

**Alternative - Unix timestamp:**
```rust
#[derive(Debug, Serialize, Schema)]
pub struct BookmarkResponse {
    pub created_at: i64,  // Unix timestamp (seconds)
}
```

**Best Practice:**
- Use `DateTime<Utc>` in your internal domain models
- Use `String` (RFC3339) in response DTOs
- Convert using `From`/`Into` traits

---

## 7. Generic Types Need Schema Trait Bounds

**Symptom:**
```
error[E0277]: the trait bound `T: RustApiSchema` is not satisfied
```

**Problem:**
```rust
#[derive(Debug, Serialize, Schema)]
pub struct PaginatedResponse<T> {  // ❌ Missing trait bound
    pub items: Vec<T>,
    pub total: usize,
}
```

**Solution:**
```rust
use rustapi_openapi::schema::RustApiSchema;

#[derive(Debug, Serialize, Schema)]
pub struct PaginatedResponse<T: RustApiSchema> {  // ✅ Trait bound added
    pub items: Vec<T>,
    pub total: usize,
    pub page: u32,
    pub limit: u32,
}
```

**Alternative - Type aliases for concrete types:**
```rust
pub type BookmarkList = PaginatedResponse<BookmarkResponse>;
pub type CategoryList = PaginatedResponse<CategoryResponse>;

async fn list_bookmarks() -> Json<BookmarkList> {
    // ...
}
```

---

## 8. `impl IntoResponse` Return Type Issues

**Problem:**
```rust
#[rustapi_rs::get("/")]
async fn handler() -> impl IntoResponse {  // ❌ May cause Handler trait errors
    Html("<h1>Hello</h1>")
}
```

**Solution - Use concrete types:**
```rust
#[rustapi_rs::get("/")]
async fn handler() -> Html<String> {  // ✅ Concrete type
    Html("<h1>Hello</h1>".to_string())
}
```

**Common Response Types:**
| Type | Use Case |
|------|----------|
| `Html<String>` | HTML content |
| `Json<T>` | JSON response (T must impl Schema) |
| `String` | Plain text |
| `StatusCode` | Status code only |
| `(StatusCode, Json<T>)` | Status + JSON |
| `Result<T, ApiError>` | Fallible responses |

---

## 9. State Not Found at Runtime

**Symptom:**
```
panic: State not found in request extensions
```

**Problem:**
```rust
#[rustapi_rs::get("/users")]
async fn list_users(State(db): State<Database>) -> Json<Vec<User>> {
    // ...
}

// main.rs
RustApi::auto()
    // ❌ Forgot to add .state(...)
    .run("0.0.0.0:8080")
    .await
```

**Solution:**
```rust
RustApi::auto()
    .state(database)  // ✅ Add the state!
    .run("0.0.0.0:8080")
    .await
```

---

## 10. Extractor Order Matters

**Rule:** Body-consuming extractors (`Json<T>`, `Body`) must come **last**.

**Wrong:**
```rust
async fn handler(
    Json(body): Json<CreateUser>,  // ❌ Body extractor first
    State(db): State<Database>,
) -> Result<Json<User>> { ... }
```

**Correct:**
```rust
async fn handler(
    State(db): State<Database>,    // ✅ Non-body extractors first
    Query(params): Query<Params>,
    Json(body): Json<CreateUser>,  // ✅ Body extractor last
) -> Result<Json<User>> { ... }
```

**Why?**
- `State`, `Query`, `Path` extract from request parts (headers, URL)
- `Json`, `Body` consume the request body (can only be read once)

---

## Quick Checklist: Adding a New Handler

- [ ] Add `Schema` derive to all extractor structs (`Query<T>`, `Path<T>`, `Json<T>`)
- [ ] Add `Schema` derive to response structs
- [ ] Use `#[rustapi_rs::get/post/...]` macros (not `rustapi_macros`)
- [ ] Add validation with `Validate` derive if needed
- [ ] Register state with `.state(...)` on `RustApi`
- [ ] Put body extractors (`Json<T>`) last in parameter list
- [ ] Run `cargo check` to verify
- [ ] Test in Swagger UI at `http://localhost:8080/docs`

---

## The Golden Rules

1. **Add `Schema` derive** to any struct used with extractors or responses
2. **Don't use `utoipa`** directly - `rustapi-openapi` is already included
3. **Import from `rustapi_rs`** only - never use internal crates directly
4. **Use `RustApi::auto()`** with handler macros for automatic route discovery

Follow these rules and you'll have a smooth experience with RustAPI! 🚀
