# RustAPI Philosophy

> *"The power of Rust. Modern DX. LLM-ready."*

---

## The Core Idea

**"API surface is ours, engines can change."**

This single principle drives every architectural decision in RustAPI. You write code against a stable, ergonomic API. We handle the complexity of HTTP protocols, async runtimes, and serialization libraries internally.

---

## Why We Built RustAPI

### The Problem

Building APIs in Rust traditionally requires:
- Understanding `hyper`'s low-level HTTP primitives
- Managing `tokio` runtime configurations
- Fighting trait bounds like `Pin<Box<dyn Future<...>>>`
- Writing boilerplate for request parsing, validation, and error handling
- Manually documenting every endpoint

**The result?** Simple APIs take 100+ lines. Developers spend more time on plumbing than business logic.

### The Solution

RustAPI provides a **facade** — a clean, stable API that wraps all the complexity:

```rust
// This is all you need. No boilerplate.
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<u64>) -> Json<User> {
    Json(User::find(id).await)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::auto().run("0.0.0.0:8080").await
}
```

5 lines. Auto-generated OpenAPI. Production-ready.

---

## Design Principles

### 1. 🎯 5-Line APIs

**Every feature must be expressible in minimal code.**

| Framework | Hello World Lines |
|-----------|-------------------|
| Raw Hyper | ~50 lines |
| Axum | ~15 lines |
| **RustAPI** | **5 lines** |

We achieve this through:
- Sensible defaults (auto-route registration, built-in Swagger UI)
- Derive macros that eliminate boilerplate
- A prelude that exports everything you need

### 2. 🛡️ Stable API Surface

**Your code depends only on `rustapi-rs`. Internal dependencies are hidden.**

```toml
# Your Cargo.toml - simple and stable
[dependencies]
rustapi-rs = "0.1"
```

You never write:
```toml
# ❌ Not this - internal details exposed
hyper = "1.0"
tokio = "1.35"
validator = "0.16"
```

**Benefits:**
- No dependency conflicts
- Simpler `Cargo.toml`
- We can upgrade internals without breaking your code

### 3. 🔄 Engines Can Change

**The facade pattern lets us swap implementations freely.**

| Component | Current Engine | Could Become |
|-----------|----------------|--------------|
| HTTP Server | `hyper 1.x` | `hyper 2.x`, `h3` (HTTP/3) |
| Async Runtime | `tokio` | `smol`, `async-std` (future) |
| Validation | `validator` | Custom engine (planned for v1.0) |
| Router | `matchit` | Custom radix tree |
| OpenAPI | `utoipa` | Native implementation |

**Example scenario:** When `hyper 2.0` releases with breaking changes:
1. We update `rustapi-core` to use `hyper 2.0`
2. We bump `rustapi-rs` to `0.2.0`
3. **Your code stays exactly the same** — just update the version

### External Dependency Reduction (Harici Bağımlılıkları Azaltma)

RustAPI already hides external crates behind internal adapters. To reduce dependency debt, we target components with stable specs and small surface areas for replacement, while keeping the public API unchanged. The playbook is:

1. Wrap dependencies with internal traits/types so behavior is defined by RustAPI.
2. Add contract tests to lock in behavior before replacing internals.
3. Ship replacements behind feature flags, then flip defaults.

Good candidates for in-house implementations:
- Validation (`validator`) → native validation engine (already planned).
- Router (`matchit`) → internal radix tree with RustAPI-specific optimizations.
- OpenAPI (`utoipa`) → native schema generator to control outputs.
- TOON format (`toon-format`) → move core format logic into `rustapi-toon`.
- Template engine (`tera`) → minimal renderer for basic HTML views.

Not near-term targets: `tokio`, `hyper`, `tower` — large, security-sensitive, and foundational crates best kept upstream for now.

### 4. 🎁 Batteries Included (But Optional)

**Everything you need, nothing you don't.**

```toml
# Just the basics
rustapi-rs = "0.1"

# Kitchen sink
rustapi-rs = { version = "0.1", features = ["full"] }

# Pick what you need
rustapi-rs = { version = "0.1", features = ["jwt", "cors", "toon"] }
```

| Feature | What You Get |
|---------|--------------|
| `jwt` | JWT authentication with `AuthUser<T>` extractor |
| `cors` | CORS middleware with builder pattern |
| `rate-limit` | IP-based rate limiting |
| `toon` | LLM-optimized TOON format |
| `swagger-ui` | Auto-generated `/docs` endpoint |
| `full` | All features enabled |

### 5. 🤖 LLM-First Design

**Built for the AI era.**

Traditional JSON:
```json
{"users":[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}],"total":2}
```
→ **~20 tokens**

TOON format:
```
users[(id:1,name:Alice)(id:2,name:Bob)]total:2
```
→ **~9 tokens** (55% savings)

RustAPI provides:
- `Toon<T>` — Direct TOON responses
- `LlmResponse<T>` — Content negotiation with token counting headers
- `AcceptHeader` — Automatic format detection

```rust
#[rustapi_rs::get("/ai/data")]
async fn ai_endpoint(accept: AcceptHeader) -> LlmResponse<Data> {
    LlmResponse::new(data, accept.preferred)
}
// Response headers: X-Token-Count-JSON, X-Token-Count-TOON, X-Token-Savings
```

---

## What We DON'T Do

### No Direct Dependency Exposure

```rust
// ❌ We don't expose hyper types
fn handler(req: hyper::Request<...>) -> hyper::Response<...>

// ✅ We provide our own abstractions
fn handler(req: Request) -> impl IntoResponse
```

### No Configuration Hell

```rust
// ❌ Not this
let server = Server::builder()
    .http1_header_max_size(8192)
    .http1_only(true)
    .tcp_keepalive(Some(Duration::from_secs(60)))
    .build(...);

// ✅ This
RustApi::new().run("0.0.0.0:8080").await
```

### No Trait Bound Nightmares

```rust
// ❌ Not this
where
    T: Service<Request<Body>, Response = Response<ResBody>> + Clone + Send + 'static,
    T::Error: Into<BoxError>,
    T::Future: Send,
    ResBody: Body<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError>,

// ✅ Just this
async fn handler(Json(body): Json<T>) -> Json<R>
```

---

## Success Metrics

| Goal | Target | Achieved |
|------|--------|----------|
| Hello World | ≤ 5 lines | ✅ 5 lines |
| CRUD tutorial | ≤ 15 min | ✅ ~10 min |
| First Swagger UI | Zero config | ✅ Auto at `/docs` |
| Compile errors | Understandable | ✅ Clear hints |
| LLM token savings | ≥ 50% | ✅ 50-58% |

---

## The Path Forward

### Short Term (v0.x)
- Polish existing features
- Performance optimizations (simd-json, better allocations)
- More middleware (compression, static files)

### Medium Term (v1.0)
- Custom validation engine (remove `validator` dependency)
- Async validation support
- Stable API guarantee

### Long Term
- WebSocket support
- GraphQL (optional crate)
- gRPC (optional crate)
- HTTP/3 via `h3` (transparent upgrade)

---

## Summary

RustAPI is not just another web framework. It's a **philosophy**:

1. **Simplicity first** — 5 lines to production
2. **Stability always** — Your code never breaks
3. **Future-ready** — Built for AI, ready for anything

```rust
use rustapi_rs::prelude::*;

// This will work in 2024, 2025, and beyond.
// Engines change. Your code doesn't.
```

---

*"API surface is ours, engines can change."*
