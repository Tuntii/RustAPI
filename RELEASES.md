# RustAPI v0.1.410 Release Notes

**Release Date**: March 9, 2026
**Full Changelog**: https://github.com/Tuntii/RustAPI/compare/v0.1.397...v0.1.410

**Benchmark Source of Truth**: Current benchmark methodology and canonical performance claims live in `docs/PERFORMANCE_BENCHMARKS.md`. Historical release-specific benchmark notes should be treated as point-in-time snapshots unless they are linked from that document.

---

## 🎯 Highlights

v0.1.410 is the **Production Baseline** release. It delivers everything you need to go from prototype to production-ready service with a single builder call — health probes, session management, rate limiting, observability tooling, and a suite of real-world examples.

| Feature | Crate | Impact |
|---------|-------|--------|
| Production Defaults Preset | `rustapi-core` | One-call production setup: health probes + tracing + request IDs |
| Health Check System | `rustapi-core` | Built-in `/health`, `/ready`, `/live` with custom checks |
| Session Management | `rustapi-extras` | Cookie-backed sessions with pluggable stores |
| Rate Limiting Strategies | `rustapi-extras` | Fixed window, sliding window, and token bucket |
| CLI: bench & observability | `cargo-rustapi` | New `bench` and `observability` subcommands |
| Multipart Streaming | `rustapi-core` | Enhanced streaming multipart with progress tracking |
| 4 New Examples | `rustapi-rs` | Auth, CRUD, Jobs, Streaming — ready to copy |
| 10+ Cookbook Recipes | `docs/cookbook` | Migration guides, session auth, observability, error handling |

---

## 🏭 Production Defaults Preset

Go production-ready with a single call:

```rust
use rustapi_rs::prelude::*;

#[tokio::main]
async fn main() {
    RustApi::new()
        .production_defaults("my-service")
        .run("0.0.0.0:3000")
        .await;
}
```

This enables:
- **`RequestIdLayer`** — unique ID on every request
- **`TracingLayer`** — structured logging with service metadata
- **`/health`**, **`/ready`**, **`/live`** — Kubernetes-compatible probes

Customizable via `ProductionDefaultsConfig`:

```rust
RustApi::new()
    .production_defaults_with_config(
        ProductionDefaultsConfig::new("my-service")
            .version("1.2.3")
            .tracing_level(tracing::Level::DEBUG)
            .request_id(true)
            .health_endpoints(true)
    )
    .run("0.0.0.0:3000")
    .await;
```

---

## 🏥 Health Check System

Full health check module with builder API, custom checks, and OpenAPI integration:

```rust
use rustapi_rs::prelude::*;

let health = HealthCheckBuilder::new(true)
    .add_check("database", || async {
        // Check database connectivity
        HealthStatus::healthy()
    })
    .add_check("redis", || async {
        HealthStatus::degraded("high latency".into())
    })
    .version("1.0.0")
    .build();

RustApi::new()
    .with_health_check(health)
    .health_endpoints()
    .run("0.0.0.0:3000")
    .await;
```

- **`/health`** — aggregated status of all checks (200 or 503)
- **`/ready`** — dependency readiness (200 or 503)
- **`/live`** — lightweight liveness probe (always 200)
- Configurable paths via `HealthEndpointConfig`
- `HealthStatus` variants: `Healthy`, `Unhealthy { reason }`, `Degraded { reason }`

---

## 🔐 Session Management

Cookie-backed session management with pluggable storage backends:

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extras::session::*;

let store = MemorySessionStore::new();

RustApi::new()
    .layer(SessionLayer::new(
        store,
        SessionConfig::default()
            .cookie_name("my_session")
            .ttl(Duration::from_secs(3600))
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Lax)
    ))
    .run("0.0.0.0:3000")
    .await;
```

Handler-side usage:

```rust
#[post("/login")]
async fn login(session: Session, body: Json<LoginRequest>) -> Result<Json<Value>> {
    session.insert("user_id", &body.user_id).await?;
    session.cycle_id().await; // CSRF protection
    Ok(Json(json!({"status": "ok"})))
}
```

- `Session` extractor with `get`, `insert`, `contains`, `destroy`, `cycle_id`
- `MemorySessionStore` built-in; `SessionStore` trait for custom backends
- Rolling sessions (refresh TTL on each access) by default
- Secure defaults: `HttpOnly`, `Secure`, `SameSite=Lax`

---

## 🚦 Rate Limiting Strategies

Three strategies for different use cases:

```rust
use rustapi_rs::extras::rate_limit::*;

// Fixed window: 100 requests per 60 seconds
RustApi::new()
    .layer(RateLimitLayer::new(100, Duration::from_secs(60))
        .strategy(RateLimitStrategy::FixedWindow))

// Sliding window: smoother distribution
    .layer(RateLimitLayer::new(100, Duration::from_secs(60))
        .strategy(RateLimitStrategy::SlidingWindow))

// Token bucket: allows bursts
    .layer(RateLimitLayer::new(100, Duration::from_secs(60))
        .strategy(RateLimitStrategy::TokenBucket))
```

- Per-IP tracking with `DashMap`
- Response headers: `X-RateLimit-Remaining`, `Retry-After`
- Returns `429 Too Many Requests` when limit exceeded

---

## 🔨 New CLI Commands

### `cargo rustapi bench`
Run the performance benchmark workflow:
```powershell
cargo rustapi bench --warmup 5 --iterations 1000
```

### `cargo rustapi observability`
Surface observability assets and check production readiness:
```powershell
cargo rustapi observability --check
```
Checks for production baseline docs, observability cookbook, benchmark script, and quality gate.

### `cargo rustapi doctor` (enhanced)
Expanded environment health checks with `--strict` mode that fails on warnings.

---

## 📄 Enhanced Multipart Streaming

`StreamingMultipart` and `StreamingMultipartField` now support:
- `.bytes_read()` — progress tracking
- `.save_to(path)` — stream directly to disk
- `.save_as(path)` — save with custom filename
- `.into_uploaded_file()` — convert to `UploadedFile`
- `.field_count()` — number of fields in the upload

---

## 📝 New Examples

Four production-ready examples added to `crates/rustapi-rs/examples/`:

| Example | Description |
|---------|-------------|
| `auth_api.rs` | Session-based authentication with login/logout/refresh |
| `full_crud_api.rs` | Complete CRUD API with `Arc<RwLock<HashMap>>` state |
| `jobs_api.rs` | Background job queue with `InMemoryBackend` |
| `streaming_api.rs` | Server-sent events (SSE) streaming |

---

## 📖 New Cookbook Recipes

- **Session Authentication** — cookie-backed auth patterns
- **Observability** — monitoring and tracing setup
- **Error Handling** — structured error responses
- **Custom Extractors** — building your own extractors
- **Middleware Debugging** — layer inspection and troubleshooting
- **Axum Migration** — step-by-step migration guide from Axum
- **Actix Migration** — step-by-step migration guide from Actix-web
- **OIDC/OAuth2 Production** — production-grade OAuth2 setup
- **Macro Attributes Reference** — complete reference for all route macro attributes

---

## 📦 Facade Re-exports

New types available in `rustapi_rs::prelude::*`:
- `ProductionDefaultsConfig`, `HealthCheck`, `HealthCheckBuilder`, `HealthCheckResult`, `HealthStatus`, `HealthEndpointConfig`

New modules in `rustapi_rs::extras::`:
- `session` — `Session`, `SessionLayer`, `SessionConfig`, `MemorySessionStore`, `SessionStore`, `SessionRecord`
- `rate_limit` — `RateLimitLayer`, `RateLimitStrategy`

---

---

# RustAPI v0.1.397 Release Notes

**Release Date**: February 26, 2026
**Full Changelog**: https://github.com/Tuntii/RustAPI/compare/v0.1.335...v0.1.397

**Benchmark Source of Truth**: Current benchmark methodology and canonical performance claims live in `docs/PERFORMANCE_BENCHMARKS.md`. Historical release-specific benchmark notes should be treated as point-in-time snapshots unless they are linked from that document.

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
