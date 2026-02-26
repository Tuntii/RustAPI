# RustAPI v0.1.397 Release Notes

**Release Date**: February 26, 2026
**Full Changelog**: https://github.com/Tuntii/RustAPI/compare/v0.1.335...v0.1.397

---

## 🎯 Highlights

This is the biggest feature release since v0.1.300. Seven major features land together, transforming RustAPI from a routing framework into a **production-ready application platform**.

| Feature | Crate | Impact |
|---------|-------|--------|
| Compile-Time Extractor Safety | `rustapi-macros` | Zero runtime surprises from body-consuming extractors |
| Typed Error Responses | `rustapi-macros` + `rustapi-core` | Errors auto-reflected in OpenAPI spec |
| Pagination & HATEOAS | `rustapi-core` | Offset + cursor pagination with RFC 8288 Link headers |
| Built-in Caching Layer | `rustapi-extras` | LRU + ETag + `304 Not Modified` out of the box |
| Event System & Lifecycle Hooks | `rustapi-core` | In-process pub/sub + `on_start` / `on_shutdown` |
| Native Hot Reload | `cargo-rustapi` | Zero-dependency file watcher, no `cargo-watch` needed |
| gRPC Support | `rustapi-grpc` | First crates.io release — dual HTTP + gRPC server |

---

## ⚡ Compile-Time Extractor Safety

Body-consuming extractors (`Json<T>`, `Body`, `ValidatedJson<T>`) **must** now be the last handler parameter. The macro emits a clear compile error instead of silently failing at runtime:

```rust
// ✅ Compiles
#[get("/users")]
async fn ok(State(db): State<Db>, body: Json<CreateUser>) -> Result<Json<User>> { ... }

// ❌ Compile error: "Body-consuming extractors must be the last parameter"
#[get("/users")]
async fn bad(body: Json<CreateUser>, State(db): State<Db>) -> Result<Json<User>> { ... }
```

Multiple body-consuming extractors in the same handler are also detected at compile time.

---

## 🏷️ Typed Error Responses (OpenAPI)

Declare possible error responses with `#[errors()]` — they appear automatically in Swagger UI:

```rust
#[get("/users/{id}")]
#[errors(404 = "User not found", 403 = "Insufficient permissions")]
async fn get_user(Path(id): Path<Uuid>) -> Result<Json<User>> { ... }
```

Programmatic alternative: `Route::error_response(404, "Not found")`.

---

## 📄 Pagination & HATEOAS

### Offset Pagination
```rust
async fn list(paginate: Paginate, State(db): State<Db>) -> Paginated<User> {
    let (users, total) = db.find_users(paginate.offset(), paginate.limit()).await;
    paginate.paginate(users, total)
}
// GET /users?page=2&per_page=20
// → { items: [...], meta: { page: 2, per_page: 20, total: 150, total_pages: 8 }, _links: {...} }
// → Link: <...?page=3&per_page=20>; rel="next", <...?page=1&per_page=20>; rel="prev"
```

### Cursor Pagination
```rust
async fn feed(cursor: CursorPaginate) -> CursorPaginated<Post> {
    let posts = db.posts_after(cursor.cursor(), cursor.limit()).await;
    cursor.after(posts, next_cursor, has_more)
}
```

Both include `X-Total-Count` and `X-Total-Pages` headers. All types in the prelude.

---

## 🗄️ Built-in Caching Layer

Full rewrite with production-grade features:

```rust
use rustapi_rs::prelude::*;

let app = RustApi::new()
    .layer(
        CacheBuilder::new()
            .max_entries(5_000)
            .default_ttl(Duration::from_secs(300))
            .vary_by(&["Accept", "Authorization"])
            .build()
    );
```

- **LRU eviction** with configurable `max_entries`
- **ETag** generation via FNV-1a hash + automatic `304 Not Modified`
- **Cache-Control** awareness (`no-cache`, `no-store`)
- **`CacheHandle`** for programmatic invalidation (by prefix, exact key, or clear all)

---

## 🔔 Event System & Lifecycle Hooks

### EventBus
```rust
let bus = EventBus::new();
bus.on("user.created", |data| { println!("New user: {data}"); });
bus.emit("user.created", "alice@example.com");
```

Supports sync and async handlers, fire-and-forget (`emit`) and await-all (`emit_await`).

### Lifecycle Hooks
```rust
RustApi::new()
    .on_start(|| async { println!("🚀 Server starting..."); Ok(()) })
    .on_shutdown(|| async { println!("👋 Graceful shutdown"); Ok(()) })
    .run("0.0.0.0:8080").await;
```

---

## 🔥 Native Hot Reload

No more `cargo install cargo-watch`:

```bash
cargo rustapi watch          # Watch mode with 300ms debounce
cargo rustapi run --watch    # Same via run subcommand
```

- **Build-before-restart**: Only restarts if `cargo build` succeeds
- **Crash recovery**: Watches for changes even after build failure
- **Smart filtering**: Ignores `target/`, `.git/`, non-Rust files
- **`.hot_reload(true)`** builder API prints dev-mode banner

---

## 🌐 gRPC Support (First Release)

`rustapi-grpc` is now on **crates.io** for the first time:

```toml
[dependencies]
rustapi-rs = { version = "0.1.397", features = ["protocol-grpc"] }
```

```rust
use rustapi_grpc::run_rustapi_and_grpc;

run_rustapi_and_grpc(http_app, grpc_router, "[::]:8080", "[::]:50051").await;
```

Re-exports `tonic` and `prost` for seamless proto integration.

---

## 🏗️ Facade Stabilization & Governance

- Public API surface explicitly curated under `core`, `protocol`, `extras`, `prelude` modules
- API snapshot files (`api/public/`) with CI drift check
- `CONTRACT.md` defining SemVer contract, MSRV (1.78), deprecation policy
- Feature taxonomy: `core-*`, `protocol-*`, `extras-*` canonical names

---

## 🔧 Fixes & Maintenance

- Clippy: `.map_or(false, ...)` → `.is_some_and(...)` in cache middleware
- Clippy: Nested `format!` → single `format!` in ETag generation
- Publish pipeline: `rustapi-grpc` added to both publish scripts

---

## 📦 Upgrade Guide

```toml
# Cargo.toml
rustapi-rs = "0.1.397"

# For new features:
rustapi-rs = { version = "0.1.397", features = ["full"] }
```

No breaking changes. Deprecated legacy feature aliases still work and will be removed no earlier than two minor releases after this announcement.

---

## What's Changed (PRs)

- Add lifecycle hooks, pagination, and cache (#139)
- Fix review feedback: deduplication, pagination validation, cache correctness (#140)
- docs: cookbook expansion and learning path improvements (#132)
- core: stabilize facade API surface, feature taxonomy, and public-api CI gate (#122)
- Add optional rustapi-grpc crate (tonic/prost) (#118)
- docs: multiple learning path and cookbook improvements (#109, #112, #120, #121, #123-126, #129)

**Full Changelog**: https://github.com/Tuntii/RustAPI/compare/v0.1.335...v0.1.397
