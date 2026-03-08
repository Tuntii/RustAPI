# Axum -> RustAPI Migration Guide

If you already know Axum, RustAPI will feel familiar in the right places and pleasantly less repetitive in a few others.

This guide focuses on the migration path for the most common Axum patterns:

- handlers and extractors
- app state
- route registration
- middleware
- testing
- OpenAPI/documentation

## What stays familiar

The good news first: most everyday handler code barely changes.

| Axum concept | RustAPI equivalent | Notes |
|---|---|---|
| `State<T>` | `State<T>` | same mental model |
| `Path<T>` | `Path<T>` | same purpose |
| `Query<T>` | `Query<T>` | same purpose |
| `Json<T>` | `Json<T>` | same purpose |
| `Router::route()` | `RustApi::route()` | similar registration flow |
| tower layers | `.layer(...)` | middleware stack support |
| integration testing with service/router | `TestClient` | in-memory, ergonomic |

The biggest differences are:

1. RustAPI encourages using `rustapi-rs` as a stable facade.
2. RustAPI can auto-discover macro-annotated routes with `RustApi::auto()`.
3. OpenAPI support is built directly into the framework flow.

## 1. Imports: switch to the facade

In Axum projects, imports are often spread across `axum`, `tower`, and OpenAPI add-ons.

In RustAPI, start from the facade:

```rust
use rustapi_rs::prelude::*;
```

That keeps your application code pinned to the public API surface instead of internal crates.

## 2. Basic handlers migrate almost directly

### Axum

```rust
use axum::{extract::Path, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct User {
    id: i64,
    name: String,
}

async fn get_user(Path(id): Path<i64>) -> Json<User> {
    Json(User {
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

- The extractor shape is essentially the same.
- Add `Schema` when you want the type represented in generated OpenAPI docs.
- RustAPI route macros use `"/users/{id}"` path syntax.

## 3. App bootstrap: `Router` -> `RustApi`

### Axum

```rust
use axum::{routing::get, Router};

let app = Router::new().route("/users/:id", get(get_user));
```

### RustAPI

```rust
use rustapi_rs::prelude::*;

let app = RustApi::new().route("/users/{id}", get(get_user));
```

### Migration note

- The conceptual shape is the same.
- Path parameters use `{id}` instead of `:id`.
- If you annotate handlers with route macros, you can often skip manual registration and use `RustApi::auto()`.

## 4. Auto-registration can replace manual route wiring

This is one of the biggest quality-of-life upgrades when moving from Axum.

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

If your Axum app has a lot of repetitive `Router::new().route(...).route(...).route(...)` setup, this is where some boilerplate quietly disappears into the floorboards.

## 5. State injection is very similar

### Axum

```rust
use axum::{extract::State, routing::get, Router};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    db: Arc<String>,
}

async fn users(State(state): State<AppState>) -> String {
    state.db.to_string()
}

let app = Router::new().route("/users", get(users)).with_state(AppState {
    db: Arc::new("db".into()),
});
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

- Keep your state `Clone + Send + Sync`.
- The usual Axum pattern of storing cheap-to-clone `Arc<_>` fields still applies nicely.

## 6. Extractor migration map

For common endpoint code, the mapping is straightforward.

| Axum | RustAPI | Notes |
|---|---|---|
| `State<T>` | `State<T>` | same pattern |
| `Path<T>` | `Path<T>` | same pattern |
| `Query<T>` | `Query<T>` | same pattern |
| `Json<T>` | `Json<T>` | same pattern |
| custom `FromRequestParts` | custom `FromRequestParts` | same idea for non-body extraction |
| custom `FromRequest` | custom `FromRequest` | use for body-consuming extractors |

### Important RustAPI rule

Body-consuming extractors such as `Json<T>`, `Body`, `ValidatedJson<T>`, and `Multipart` must be the **last** handler parameter.

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

## 7. Middleware: tower mindset, RustAPI entry point

If you are coming from Axum middleware, the main mental model still fits: request goes in, response comes out, layers wrap handlers.

Apply middleware with:

```rust,ignore
RustApi::new()
    .layer(SimpleLogger)
    .route("/users", get(users));
```

### Migration note

- The middleware shape is not a drop-in copy of Axum’s tower APIs.
- For simple request/response transformations, prefer RustAPI interceptors when they are sufficient; they are lighter than a full middleware layer.
- For a dedicated middleware walkthrough, see [Custom Middleware](custom_middleware.md).

## 8. Error handling becomes more uniform

Axum applications often build custom response tuples or custom error enums. That still works conceptually, but RustAPI leans toward `ApiError` for the common cases.

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

- `#[errors(...)]` documents the OpenAPI surface.
- Your handler still needs to return the actual runtime error.
- In production, RustAPI masks internal 5xx details automatically.

## 9. OpenAPI is no longer a side quest

In Axum, OpenAPI commonly arrives through extra libraries and extra setup.

In RustAPI, it is part of the main story:

- derive `Schema` for DTOs
- annotate handlers with `#[get]`, `#[post]`, etc.
- optionally add `#[tag]`, `#[summary]`, `#[description]`, `#[param]`, and `#[errors]`
- serve docs automatically through the app flow

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

If you are migrating from Axum plus a third-party OpenAPI stack, consolidating those concerns in one framework usually makes the codebase easier to explain to Future You™.

## 10. Testing migration: service tests -> `TestClient`

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

- `TestClient` exercises the app in memory, without binding a socket.
- This is a good destination for many Axum integration tests that currently go through a service stack manually.

## 11. Practical migration checklist

Use this order for a low-drama migration:

1. Replace Axum imports with `rustapi_rs::prelude::*` where possible.
2. Change route path syntax from `:id` to `{id}`.
3. Move shared dependencies into `State<T>`.
4. Convert handlers one endpoint at a time.
5. Add `Schema` derives to DTOs that should appear in OpenAPI.
6. Replace manual route tables with route macros and `RustApi::auto()` when it reduces boilerplate.
7. Port middleware selectively instead of all at once.
8. Replace service-level tests with `TestClient` where it simplifies setup.

## 12. A small before/after mental model

### Axum mindset

- compose a `Router`
- attach routes manually
- bolt on docs separately
- manage state and layers around the router

### RustAPI mindset

- write handler-first code
- annotate routes directly
- let `RustApi::auto()` discover them when useful
- keep docs and route metadata close to the handler

## Related reading

- [Macro Attribute Reference](../reference/macro_attributes.md)
- [Custom Extractors](custom_extractors.md)
- [Error Handling](error_handling.md)
- [Middleware Debugging](middleware_debugging.md)