# Actix-web -> RustAPI Migration Guide

If you already know Actix-web, RustAPI will feel familiar in a few core areas while removing some of the ceremony around route registration and OpenAPI integration.

This guide focuses on the migration path for the most common Actix-web patterns:

- handlers and extractors
- app state
- route registration
- middleware
- testing
- OpenAPI/documentation

## What stays familiar

The good news first: the everyday endpoint concepts map cleanly.

| Actix-web concept | RustAPI equivalent | Notes |
|---|---|---|
| `web::Data<T>` | `State<T>` | shared application state |
| `web::Path<T>` | `Path<T>` | typed path extraction |
| `web::Query<T>` | `Query<T>` | typed query extraction |
| `web::Json<T>` | `Json<T>` | JSON body extraction |
| `App::route()` / `.service()` | `RustApi::route()` / route macros | both support explicit routing |
| `wrap(...)` middleware | `.layer(...)` | middleware stack support |
| `actix_web::test` helpers | `rustapi_testing::TestClient` | in-memory HTTP-style tests |

The biggest differences are:

1. RustAPI encourages application code to import from the `rustapi-rs` facade.
2. RustAPI can auto-discover macro-annotated routes with `RustApi::auto()`.
3. OpenAPI support is designed to live close to handlers instead of being bolted on later.

## 1. Imports: switch to the facade

Actix-web applications usually import directly from `actix_web`.

In RustAPI, start from the public facade:

```rust
use rustapi_rs::prelude::*;
```

That keeps your application code aligned with RustAPI’s stable public surface instead of internal implementation crates.

## 2. Basic handlers migrate directly

### Actix-web

```rust
use actix_web::{get, web, Responder};
use serde::Serialize;

#[derive(Serialize)]
struct User {
    id: i64,
    name: String,
}

#[get("/users/{id}")]
async fn get_user(id: web::Path<i64>) -> impl Responder {
    let id = id.into_inner();

    web::Json(User {
        id,
        name: "Alice".into(),
    })
}
```

### RustAPI

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize, Schema)]
struct User {
    id: i64,
    name: String,
}

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<i64>) -> Json<User> {
    Json(User {
        id,
        name: "Alice".into(),
    })
}
```

### Migration note

- The path syntax is already `{id}` in both ecosystems, so that part stays pleasantly boring.
- Add `Schema` when the type should appear in generated OpenAPI docs.
- RustAPI handler signatures stay compact and keep extractor types explicit.

## 3. App bootstrap: `App` -> `RustApi`

### Actix-web

```rust
use actix_web::{web, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().route("/users/{id}", web::get().to(get_user))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

### RustAPI

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::new()
        .route("/users/{id}", get(get_user))
        .run("127.0.0.1:8080")
        .await
}
```

### Migration note

- `RustApi::new()` is the main application entry point.
- `RustApi::route()` is the closest equivalent to explicit Actix route registration.
- For macro-annotated handlers, `RustApi::auto()` can remove repetitive wiring.

## 4. Auto-registration can replace repetitive `.service(...)`

If your Actix-web app registers many handlers manually, RustAPI can let the route macros do more of the work.

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/health")]
async fn health() -> &'static str {
    "ok"
}

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<i64>) -> Json<i64> {
    Json(id)
}

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::auto().run("127.0.0.1:8080").await
}
```

This is where a wall of `.service(...)` calls starts to quietly disappear. Your future diff reviews may even send a thank-you card.

## 5. State injection: `web::Data<T>` -> `State<T>`

### Actix-web

```rust
use actix_web::{web, App, HttpServer, Responder};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    db: Arc<String>,
}

async fn users(state: web::Data<AppState>) -> impl Responder {
    state.db.to_string()
}

let state = AppState {
    db: Arc::new("db".into()),
};
```

### RustAPI

```rust
use rustapi_rs::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    db: Arc<String>,
}

#[rustapi_rs::get("/users")]
async fn users(State(state): State<AppState>) -> String {
    state.db.to_string()
}

let app = RustApi::new()
    .state(AppState {
        db: Arc::new("db".into()),
    })
    .route("/users", get(users));
```

### Migration note

- Keep shared state `Clone + Send + Sync`.
- Cheap-to-clone `Arc<_>` fields remain the right pattern for shared dependencies.
- Instead of wrapping state in `web::Data<T>`, RustAPI stores the state directly and extracts it with `State<T>`.

## 6. Extractor migration map

| Actix-web | RustAPI | Notes |
|---|---|---|
| `web::Data<T>` | `State<T>` | shared app state |
| `web::Path<T>` | `Path<T>` | typed path extraction |
| `web::Query<T>` | `Query<T>` | typed query extraction |
| `web::Json<T>` | `Json<T>` | body extraction |
| custom request extractor | `FromRequestParts` / `FromRequest` | choose based on body usage |

### Important RustAPI rule

Body-consuming extractors such as `Json<T>`, `Body`, `ValidatedJson<T>`, `AsyncValidatedJson<T>`, and `Multipart` must be the **last** handler parameter.

```rust
#[rustapi_rs::post("/users/{id}")]
async fn update_user(
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
    Json(_body): Json<User>,
) -> Result<()> {
    Ok(())
}
```

## 7. Middleware: `wrap(...)` mindset, RustAPI entry point

Actix-web middleware and RustAPI middleware share the same big-picture mental model: requests go in, responses come out, and the middleware stack wraps the handler.

Apply middleware with:

```rust,ignore
RustApi::new()
    .layer(RequestIdLayer::new())
    .layer(TracingLayer::new())
    .route("/users", get(users));
```

### Migration note

- Use `.layer(...)` for full middleware wrapping behavior.
- For lightweight request/response transformations, prefer interceptors when they are sufficient; they are cheaper than full middleware.
- Middleware layering order matters, so keep observability/auth/retry ordering intentional.

## 8. Error handling becomes more uniform

Actix-web often leans on `ResponseError`, `HttpResponse`, or custom response builders. RustAPI keeps the same flexibility, but the common path is `ApiError`.

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/users/{id}")]
#[rustapi_rs::errors(404 = "User not found")]
async fn get_user(Path(id): Path<i64>) -> Result<Json<User>> {
    if id == 0 {
        return Err(ApiError::not_found("User not found"));
    }

    Ok(Json(User {
        id,
        name: "Alice".into(),
    }))
}
```

### Migration note

- `#[errors(...)]` documents the OpenAPI response surface.
- Your handler still needs to return the matching runtime error.
- In production, RustAPI masks internal 5xx details automatically.

## 9. OpenAPI moves closer to the handler

In Actix-web projects, OpenAPI is often layered in through separate crates and extra registration code.

In RustAPI, it becomes part of the main handler workflow:

- derive `Schema` for DTOs
- annotate handlers with `#[get]`, `#[post]`, and friends
- optionally add `#[tag]`, `#[summary]`, `#[description]`, `#[param]`, and `#[errors]`
- serve docs through the app configuration

```rust
#[derive(Serialize, Schema)]
struct User {
    id: i64,
    name: String,
}

#[rustapi_rs::get("/users/{id}")]
#[rustapi_rs::tag("Users")]
#[rustapi_rs::summary("Get user by ID")]
#[rustapi_rs::errors(404 = "User not found")]
async fn get_user(Path(id): Path<i64>) -> Result<Json<User>> {
    Ok(Json(User {
        id,
        name: "Alice".into(),
    }))
}
```

Ordinary path and query parameters are inferred into OpenAPI automatically, so `#[param(...)]` is mainly for path-parameter schema overrides.

## 10. Testing migration: `actix_web::test` -> `TestClient`

### RustAPI test style

```rust
use rustapi_rs::prelude::*;
use rustapi_testing::TestClient;

#[rustapi_rs::get("/hello")]
async fn hello() -> &'static str {
    "hello"
}

#[tokio::test]
async fn test_hello() {
    let app = RustApi::new().route("/hello", get(hello));
    let client = TestClient::new(app);

    let response = client.get("/hello").send().await;

    assert_eq!(response.status(), 200);
}
```

### Migration note

- `TestClient` exercises the application in memory without binding a socket.
- This is a good replacement for many Actix integration tests that currently build `App` instances plus test harness glue.

## 11. Practical migration checklist

Use this order for a low-drama migration:

1. Replace handler imports with `use rustapi_rs::prelude::*` on the RustAPI side.
2. Port shared dependencies from `web::Data<T>` to `State<T>`.
3. Convert handlers one endpoint at a time.
4. Add `Schema` derives to DTOs that should appear in OpenAPI.
5. Replace repetitive `.service(...)` registration with route macros and `RustApi::auto()` when it reduces boilerplate.
6. Port middleware selectively instead of all at once.
7. Replace Actix test harness setup with `TestClient` where it simplifies coverage.
8. Add production defaults, tracing, and health probes once the endpoint layer is stable.

## 12. Mental model shift

### Actix-web mindset

- build an `App`
- register routes and services explicitly
- add middleware with `wrap(...)`
- extend docs/testing with adjacent tooling

### RustAPI mindset

- write handler-first code
- annotate routes directly
- let `RustApi::auto()` discover them when useful
- keep docs and route metadata close to handlers

## Related reading

- [Macro Attribute Reference](../reference/macro_attributes.md)
- [Custom Extractors](custom_extractors.md)
- [Error Handling](error_handling.md)
- [Middleware Debugging](middleware_debugging.md)
- [Recommended Production Baseline](../../../PRODUCTION_BASELINE.md)
