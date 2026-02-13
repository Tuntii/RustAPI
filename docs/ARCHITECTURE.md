# RustAPI Architecture

> Deep dive into RustAPI's internal structure and design decisions.

---

## Overview

RustAPI uses a **layered facade architecture** where complexity is hidden behind clean abstractions. Users interact only with `rustapi-rs`, while internal crates handle specific concerns.

```
┌─────────────────────────────────────────────────────────────────┐
│                      Your Application                           │
│                    use rustapi_rs::prelude::*                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    rustapi-rs (Public Facade)                   │
│  • Exports prelude                                              │
│  • Re-exports all public types                                  │
│  • Feature flag management                                      │
└─────────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  rustapi-core   │ │ rustapi-macros  │ │ rustapi-openapi │
│  HTTP Engine    │ │ Proc Macros     │ │ Swagger/OpenAPI │
└─────────────────┘ └─────────────────┘ └─────────────────┘
          │
          ├─────────────────┬─────────────────┬─────────────────┐
          ▼                 ▼                 ▼                 ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│rustapi-validate │ │  rustapi-toon   │ │ rustapi-extras  │ │   rustapi-ws    │
│  Validation     │ │  LLM Format     │ │ JWT/CORS/Rate   │ │   WebSocket     │
└─────────────────┘ └─────────────────┘ └─────────────────┘ └─────────────────┘
          │                 │                 │                 │
          └─────────────────┴─────────────────┴─────────────────┤
                              │                                 ▼
                              │                       ┌─────────────────┐
                              │                       │  rustapi-view   │
                              │                       │ Template Engine │
                              │                       └─────────────────┘
                              │                                 │
                              └─────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Foundation Layer                             │
│  tokio │ hyper │ serde │ matchit │ tower │ tungstenite │ tera  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Crate Responsibilities

### `rustapi-rs` — Public Facade

**The only crate users import.**

```rust
// This is all users need
use rustapi_rs::prelude::*;
```

Responsibilities:
- Re-export all public types from internal crates
- Manage feature flags (`core-*`, `protocol-*`, `extras-*`)
- Provide version stability guarantees
- Documentation entry point

Facade module layout:
- `rustapi_rs::core` - Stable routing/handler/extractor/response/middleware surface.
- `rustapi_rs::protocol` - Optional protocol modules (`toon`, `ws`, `view`, `grpc`, `http3`).
- `rustapi_rs::extras` - Optional production middleware/integration modules.
- `rustapi_rs::__private` - Macro/runtime wiring (not part of public contract).

```rust
// rustapi-rs/src/lib.rs (simplified)
pub mod prelude {
    pub use rustapi_core::{
        RustApi, Json, Path, Query, State, Body,
        get, post, put, patch, delete,
        ApiError, Result,
    };
    pub use rustapi_macros::*;
    pub use rustapi_openapi::Schema;
    pub use rustapi_validate::Validate;
    
    #[cfg(feature = "protocol-toon")]
    pub use crate::protocol::toon::{Toon, LlmResponse, AcceptHeader};
    
    #[cfg(feature = "extras-jwt")]
    pub use crate::extras::jwt::*;
    
    #[cfg(feature = "protocol-ws")]
    pub use crate::protocol::ws::{WebSocket, WebSocketUpgrade, WebSocketStream, Message, Broadcast};
    
    #[cfg(feature = "protocol-view")]
    pub use crate::protocol::view::{Templates, View, ContextBuilder};
}
```

### `rustapi-core` — HTTP Engine

**The heart of the framework.**

| Component | Implementation | Purpose |
|-----------|----------------|---------|
| `RustApi` | Builder pattern | Application configuration |
| `Router` | `matchit` radix tree | URL routing |
| `Handler` | Generic trait | Request handling |
| `Extractors` | `FromRequest` trait | Type-safe parameter extraction |
| `Responses` | `IntoResponse` trait | Response building |
| `Server` | `hyper 1.x` | HTTP protocol handling |

**Key files:**
- [app.rs](../crates/rustapi-core/src/app.rs) — `RustApi` builder
- [router.rs](../crates/rustapi-core/src/router.rs) — Radix tree routing
- [handler.rs](../crates/rustapi-core/src/handler.rs) — Handler trait
- [extract.rs](../crates/rustapi-core/src/extract.rs) — All extractors
- [response.rs](../crates/rustapi-core/src/response.rs) — Response types
- [server.rs](../crates/rustapi-core/src/server.rs) — Hyper server

### `rustapi-macros` — Procedural Macros

**Compile-time code generation.**

| Macro | Purpose | Example |
|-------|---------|---------|
| `#[rustapi_rs::get]` | GET route registration | `#[rustapi_rs::get("/users")]` |
| `#[rustapi_rs::post]` | POST route registration | `#[rustapi_rs::post("/users")]` |
| `#[rustapi_rs::main]` | Async main wrapper | `#[rustapi_rs::main]` |
| `#[derive(Schema)]` | OpenAPI schema generation | `#[derive(Schema)]` |

The macros enable zero-config routing:

```rust
// This macro registers the route at compile time
#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<u64>) -> Json<User> { ... }

// RustApi::auto() collects all registered routes
RustApi::auto().run("0.0.0.0:8080").await
```

### `rustapi-openapi` — API Documentation

**Automatic OpenAPI/Swagger generation.**

Features:
- Native OpenAPI 3.1 model and schema registry (no external OpenAPI generator dependency)
- Integrity validation for component references (`$ref`)
- Serves Swagger UI at `/docs`
- Extracts schemas from `Json<T>`, `Query<T>`, `Path<T>`

```rust
// Schema derive generates OpenAPI schema
#[derive(Serialize, Schema)]
struct User {
    id: u64,
    name: String,
    #[schema(format = "email")]
    email: String,
}
```

### `rustapi-validate` — Request Validation

**Type-safe request validation.**

Features:
- Native v2 validation engine is the default (`rustapi_macros::Validate`)
- `ValidatedJson<T>` and `AsyncValidatedJson<T>` extractors
- Legacy `validator` compatibility is optional via `core-legacy-validator`
- Automatic 422 responses with field errors
- Custom validation rules

```rust
#[derive(Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 3, max = 50))]
    name: String,
    
    #[validate(email)]
    email: String,
    
    #[validate(range(min = 0, max = 150))]
    age: u8,
}

async fn create(ValidatedJson(user): ValidatedJson<CreateUser>) -> Json<User> {
    // `user` is guaranteed valid here
}
```

### `rustapi-toon` — LLM Optimization

**Token-Oriented Object Notation for AI.**

| Type | Purpose |
|------|---------|
| `Toon<T>` | Direct TOON response |
| `LlmResponse<T>` | Content negotiation + headers |
| `AcceptHeader` | Parse Accept header preference |
| `ToonFormat` | TOON serialization |

Headers provided by `LlmResponse`:
- `X-Token-Count-JSON` — Tokens if JSON
- `X-Token-Count-TOON` — Tokens if TOON  
- `X-Token-Savings` — Percentage saved
- `X-Format-Used` — Which format was used

### `rustapi-extras` — Production Middleware

**Battle-tested middleware components.**

| Component | Feature Flag | Purpose |
|-----------|--------------|---------|
| JWT Auth | `extras-jwt` | `AuthUser<T>` extractor, `JwtLayer` |
| CORS | `extras-cors` | `CorsLayer` with builder |
| Rate Limit | `extras-rate-limit` | IP-based throttling |
| Body Limit | default | Max request body size |
| Request ID | default | Unique request tracking |
| **Replay** | `extras-replay` | Request capture + replay diagnostics |
| **Circuit Breaker** | default | Fault tolerance patterns |
| **Retry** | default | Automatic retry with backoff |

### `rustapi-jobs` — Background Job Processing ⭐ NEW

**Async job queue with multiple backends.**

| Component | Purpose |
|-----------|---------|
| `Job` | Job definition with payload |
| `JobQueue` | Queue management and dispatch |
| `MemoryBackend` | In-memory store (dev/testing) |
| `RedisBackend` | Redis-backed persistence |
| `PostgresBackend` | Postgres-backed persistence |

Features:
- Retry logic with exponential backoff
- Dead letter queue for failed jobs
- Scheduled and delayed execution
- Job status tracking

### `rustapi-testing` — Test Utilities ⭐ NEW

**Helpers for integration and unit testing.**

| Type | Purpose |
|------|---------|
| `TestServer` | Spawn test server instance |
| `Matcher` | Response body/header matching |
| `Expectation` | Fluent assertion builder |

### `rustapi-ws` — WebSocket Support

**Real-time bidirectional communication.**

| Type | Purpose |
|------|---------|
| `WebSocket` | Extractor for WebSocket upgrades |
| `WebSocketUpgrade` | Response type for upgrade handshake |
| `WebSocketStream` | Async stream for send/recv |
| `Message` | Text, Binary, Ping, Pong, Close |
| `Broadcast` | Pub/sub channel for broadcasting |

### `rustapi-view` — Template Engine

**Server-side HTML rendering with Tera.**

| Type | Purpose |
|------|---------|
| `Templates` | Template engine instance |
| `View<T>` | Response type with template rendering |
| `ContextBuilder` | Build template context |
| `TemplatesConfig` | Configuration (directory, extension) |

---

## Request Flow

### 1. Incoming Request

```
HTTP Request → Hyper → RustAPI Server
```

### 2. Routing

```rust
// Router uses matchit for O(log n) route matching
let router = Router::new()
    .route("/users", get(list_users))
    .route("/users/{id}", get(get_user))
    .route("/users", post(create_user));

// matchit converts {id} to :id internally
// Radix tree enables fast prefix matching
```

### 3. Middleware Stack

```
Request → [RequestId] → [CORS] → [RateLimit] → [JWT] → [BodyLimit] → Handler
```

Each middleware can:
- Modify the request
- Short-circuit with a response
- Pass to the next layer

### 4. Extraction

```rust
async fn handler(
    Path(id): Path<u64>,           // From URL path
    Query(params): Query<Params>,   // From query string
    Json(body): Json<Body>,         // From request body
    State(db): State<DbPool>,       // From app state
) -> impl IntoResponse
```

Extractors implement `FromRequest` or `FromRequestParts`:

```rust
#[async_trait]
pub trait FromRequest: Sized {
    type Rejection: IntoResponse;
    
    async fn from_request(req: Request) -> Result<Self, Self::Rejection>;
}
```

### 5. Handler Execution

```rust
// Handlers are async functions with any number of extractors
async fn get_user(Path(id): Path<u64>) -> Json<User> {
    let user = db.find_user(id).await;
    Json(user)
}
```

### 6. Response Building

```rust
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

// Implemented for common types
impl<T: Serialize> IntoResponse for Json<T> { ... }
impl IntoResponse for &'static str { ... }
impl IntoResponse for StatusCode { ... }
impl<T> IntoResponse for (StatusCode, T) where T: IntoResponse { ... }
```

---

## Handler System

### The Handler Trait

```rust
pub trait Handler<T>: Clone + Send + Sync + 'static {
    fn call(self, req: Request) -> impl Future<Output = Response> + Send;
}
```

### Implementations (0-5 arguments)

```rust
// Zero arguments
impl<F, Fut, R> Handler<()> for F
where
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = R> + Send,
    R: IntoResponse,
{ ... }

// One argument
impl<F, Fut, R, T1> Handler<(T1,)> for F
where
    F: Fn(T1) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = R> + Send,
    R: IntoResponse,
    T1: FromRequest,
{ ... }

// ... up to 5 arguments
```

### Type Erasure with BoxedHandler

```rust
pub struct BoxedHandler {
    inner: Arc<dyn ErasedHandler>,
}

// Allows storing different handler types in the same router
```

---

## Error Handling

### ApiError Structure

```rust
pub struct ApiError {
    pub status: u16,
    pub error_type: String,
    pub message: String,
    pub error_id: String,        // Unique ID for tracking
    pub fields: Option<Vec<FieldError>>,  // Validation errors
}
```

### Error Response Format

```json
{
    "status": 422,
    "error_type": "validation_error",
    "message": "Request validation failed",
    "error_id": "err_abc123",
    "fields": [
        {
            "field": "email",
            "message": "Invalid email format"
        }
    ]
}
```

### Error Masking (Production)

```rust
// In production, internal errors are masked
// Set RUSTAPI_ENV=production

// Development: Full error details
// Production: "Internal server error" + error_id for logs
```

---

## State Management

### Application State

```rust
let app = RustApi::new()
    .state(db_pool)    // Any Clone + Send + Sync type
    .state(config)
    .route("/users", get(list_users));

// Extract in handlers
async fn list_users(State(db): State<DbPool>) -> Json<Vec<User>> {
    let users = db.query("SELECT * FROM users").await;
    Json(users)
}
```

### State is Type-Safe

```rust
// Each state type is separate
async fn handler(
    State(db): State<DbPool>,
    State(cache): State<RedisPool>,
    State(config): State<AppConfig>,
) -> impl IntoResponse
```

---

## Testing

### TestClient

```rust
use rustapi_rs::test::TestClient;

#[tokio::test]
async fn test_get_user() {
    let app = RustApi::new()
        .route("/users/{id}", get(get_user));
    
    let client = TestClient::new(app);
    
    let res = client.get("/users/1").send().await;
    assert_eq!(res.status(), 200);
    
    let user: User = res.json().await;
    assert_eq!(user.id, 1);
}
```

### Testing with State

```rust
#[tokio::test]
async fn test_with_mock_db() {
    let mock_db = MockDb::new();
    mock_db.insert(User { id: 1, name: "Test".into() });
    
    let app = RustApi::new()
        .state(mock_db)
        .route("/users/{id}", get(get_user));
    
    let client = TestClient::new(app);
    // ...
}
```

---

## Performance Considerations

### Current Optimizations

| Area | Technique |
|------|-----------|
| Routing | Radix tree (O(log n) lookup) |
| Serialization | Zero-copy where possible |
| Allocation | Response buffer pre-allocation |
| Async | Tokio work-stealing scheduler |

### Planned Optimizations

| Area | Technique | Expected Gain |
|------|-----------|---------------|
| JSON Parsing | `core-simd-json` feature flag | 2-4x faster |
| Path Params | `SmallVec<[_; 4]>` | Stack-optimized, fewer allocations |
| Tracing | Conditional compilation | 10-20% less overhead |
| String Handling | Path borrowing | Fewer copies |
| Streaming Body | Unbuffered request body | Memory efficient for large uploads |

---

## Testing Architecture

### Property-Based Testing

RustAPI uses `proptest` for property-based testing of critical components:

| Test Suite | Validates |
|------------|-----------|
| Streaming Memory | Memory bounds during streaming |
| Audit Events | Field completeness and serialization |
| CSRF Tokens | Token lifecycle and uniqueness |
| OAuth2 Tokens | Token exchange round-trips |
| OpenTelemetry | Trace context propagation |
| Structured Logging | Log format compliance |

```rust
// Example property test
proptest! {
    #[test]
    fn streaming_respects_memory_bounds(data: Vec<u8>) {
        // Property: streaming never exceeds configured limit
        prop_assert!(stream_memory_usage(&data) <= MAX_BUFFER_SIZE);
    }
}
```

---

## Security Model

### Built-in Protections

| Protection | Default | Configurable |
|------------|---------|--------------|
| Body size limit | 1 MB | Yes |
| Error masking | Production only | Yes |
| Request ID | Always | Yes |
| Timeout | 30s | Yes |

### JWT Integration

```rust
let app = RustApi::new()
    .layer(JwtLayer::new("secret").skip_paths(["/health", "/login"]))
    .route("/protected", get(protected_handler));

async fn protected_handler(user: AuthUser<Claims>) -> Json<Response> {
    // `user.claims` is the decoded JWT
}
```

---

## Extending RustAPI

### Custom Extractors

```rust
pub struct ClientIp(pub IpAddr);

#[async_trait]
impl FromRequestParts for ClientIp {
    type Rejection = ApiError;
    
    async fn from_request_parts(parts: &mut Parts) -> Result<Self, Self::Rejection> {
        // Extract from X-Forwarded-For or socket addr
        let ip = extract_client_ip(parts)?;
        Ok(ClientIp(ip))
    }
}
```

### Custom Responses

```rust
pub struct Xml<T>(pub T);

impl<T: Serialize> IntoResponse for Xml<T> {
    fn into_response(self) -> Response {
        let body = quick_xml::se::to_string(&self.0).unwrap();
        Response::builder()
            .header("Content-Type", "application/xml")
            .body(body.into())
            .unwrap()
    }
}
```

### Custom Middleware

```rust
pub struct TimingLayer;

impl<S> Layer<S> for TimingLayer {
    type Service = TimingService<S>;
    
    fn layer(&self, service: S) -> Self::Service {
        TimingService { inner: service }
    }
}
```

---

## Summary

RustAPI's architecture enables:

1. **Simplicity** — One import, minimal boilerplate
2. **Safety** — Compile-time type checking, no runtime surprises
3. **Flexibility** — Extend with custom extractors, responses, middleware
4. **Performance** — Zero-cost abstractions where possible
5. **Stability** — Internal changes don't break user code

The facade pattern is the key: `rustapi-rs` provides a stable surface, while internal crates can evolve freely.

---

## Workspace Structure & API Surface

### Public Surface

- **Public Crates**:
  - `rustapi-rs`: Main framework entry point (Facade).
  - `cargo-rustapi`: CLI tool.
- **Internal/Support Crates**:
  - `rustapi-core`, `rustapi-macros`, `rustapi-validate`;
  - `rustapi-openapi`, `rustapi-extras`, `rustapi-toon`, `rustapi-grpc`;
  - `rustapi-ws`, `rustapi-view`, `rustapi-testing`, `rustapi-jobs`.

### Semver Policy

- **Facade Contract**: `rustapi-rs` is the compatibility boundary and follows strict SemVer rules.
- **Internal Crates**: implementation details; APIs may change without facade guarantees.
- **Source of truth**: see `CONTRACT.md` for SemVer/MSRV/deprecation policy.

### Workspace Members

12 Library Crates + 2 Bench suites + 1 CLI (`crates/cargo-rustapi`).
