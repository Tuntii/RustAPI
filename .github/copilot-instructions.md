# RustAPI Copilot Instructions

## Architecture Overview

RustAPI uses a **layered facade architecture**. Users import only `rustapi-rs` which re-exports types from internal crates:

```
rustapi-rs (facade) → rustapi-core, rustapi-macros, rustapi-openapi, rustapi-validate
                   → Optional: rustapi-extras, rustapi-toon, rustapi-ws, rustapi-view
```

**Key principle:** "API surface is ours, engines can change." Internal dependencies (hyper, tokio, matchit) are wrapped—never exposed to users.

## Crate Responsibilities

| Crate | Purpose |
|-------|---------|
| `rustapi-rs` | Public facade, re-exports, feature flags |
| `rustapi-core` | HTTP engine, routing (matchit), extractors, `RustApi` builder |
| `rustapi-macros` | `#[rustapi_rs::get/post/...]`, `#[rustapi_rs::schema]` |
| `rustapi-openapi` | OpenAPI spec generation, Swagger UI |
| `rustapi-validate` | `ValidatedJson<T>`, validator integration |
| `rustapi-extras` | JWT (`jwt`), CORS (`cors`), rate-limit (`rate-limit`) |
| `rustapi-toon` | LLM-optimized TOON format (`toon`) |
| `rustapi-ws` | WebSocket support (`ws`) |
| `rustapi-view` | Tera templates (`view`) |

## Code Patterns

### Handler Pattern (5 lines philosophy)
```rust
#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<u64>) -> Json<User> {
    Json(User { id, name: "Alice".into() })
}
```

### Zero-Config Auto-Registration
Use `RustApi::auto()` to auto-discover `#[rustapi_rs::get/post/...]` decorated handlers. Manual: `RustApi::new().route(...)`.

### Extractors implement `FromRequest`/`FromRequestParts`
- `Json<T>`, `Path<T>`, `Query<T>`, `State<T>`, `Body`, `Headers`, `ValidatedJson<T>`
- Each extracts from request and can fail with `ApiError`

### Responses implement `IntoResponse`
- `Json<T>`, `Created<T>`, `NoContent`, `Html<T>`, `Redirect`, `&'static str`
- Tuples: `(StatusCode, T)`, `(StatusCode, [(header, value)], T)`

## Development Commands

```bash
# Build everything
cargo build --workspace --all-features

# Test everything  
cargo test --workspace --all-features

# Format (required before PR)
cargo fmt --all

# Lint (must pass -D warnings)
cargo clippy --workspace --all-features -- -D warnings

# Run examples
cargo run -p hello-world
cargo run -p crud-api
cargo run -p auth-api
```

## Publishing (Dependency Order)

Crates must be published in order due to dependencies:
1. `rustapi-macros` → 2. `rustapi-validate` → 3. `rustapi-openapi` → 4. `rustapi-core`
5. `rustapi-extras` → 6. `rustapi-toon` → 7. `rustapi-ws` → 8. `rustapi-view`
9. `rustapi-rs` → 10. `cargo-rustapi`

Use `scripts/smart_publish.ps1` for automated version-aware publishing.

## Key Implementation Details

### Router: Uses `matchit` radix tree
- Path params: `{id}` in macros → `:id` internally
- Route conflicts detected at startup

### Handler trait with type erasure
```rust
pub trait Handler<T>: Clone + Send + Sync + 'static {
    fn call(self, req: Request) -> impl Future<Output = Response> + Send;
}
```
Implemented for functions with 0-5 extractors. `BoxedHandler` erases types for router storage.

### Error Handling
```rust
pub struct ApiError { status, error_type, message, error_id, fields }
```
- Use `ApiError::not_found()`, `bad_request()`, etc.
- Production (`RUSTAPI_ENV=production`) masks internal errors

### Feature Flags
When adding features, gate with `#[cfg(feature = "...")]` and update `rustapi-rs/Cargo.toml`.

## File Locations

- **Core types:** `crates/rustapi-core/src/{extract,response,handler,router,app}.rs`
- **Macros:** `crates/rustapi-macros/src/lib.rs`
- **Examples:** `examples/*/src/main.rs`
- **Tests:** `crates/*/tests/` or inline `#[cfg(test)]`
- **Docs:** `docs/{ARCHITECTURE,FEATURES,GETTING_STARTED}.md`
