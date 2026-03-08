# Session-Based Authentication

Cookie-backed session auth is the shortest path from “I need login/logout” to a production-shaped RustAPI service.

This recipe shows how to:

- load a session from a cookie before your handler runs,
- read and mutate session data through the `Session` extractor,
- rotate the session ID on login / refresh,
- swap the store backend from memory to Redis without changing handler code.

## Prerequisites

Enable the session feature on the public facade.

```toml
[dependencies]
rustapi-rs = { version = "0.1.389", features = ["extras-session"] }
```

If you want Redis-backed sessions, add the Redis backend feature too:

```toml
[dependencies]
rustapi-rs = { version = "0.1.389", features = ["extras-session", "extras-session-redis"] }
```

## Solution

`rustapi-rs` now exposes the full session flow through the facade.

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extras::session::{MemorySessionStore, Session, SessionConfig, SessionLayer};
use std::time::Duration;

#[derive(Debug, Deserialize, Schema)]
struct LoginRequest {
    user_id: String,
}

#[derive(Debug, Serialize, Schema)]
struct SessionView {
    authenticated: bool,
    user_id: Option<String>,
    refreshed: bool,
    session_id: Option<String>,
}

async fn session_view(session: &Session) -> SessionView {
    let user_id = session.get::<String>("user_id").await.ok().flatten();
    let refreshed = session
        .get::<bool>("refreshed")
        .await
        .ok()
        .flatten()
        .unwrap_or(false);

    SessionView {
        authenticated: user_id.is_some(),
        user_id,
        refreshed,
        session_id: session.id().await,
    }
}

async fn login(session: Session, Json(payload): Json<LoginRequest>) -> Json<SessionView> {
    session.cycle_id().await;
    session.insert("user_id", &payload.user_id).await.expect("session insert");
    session.insert("refreshed", false).await.expect("session insert");
    Json(session_view(&session).await)
}

async fn me(session: Session) -> Json<SessionView> {
    Json(session_view(&session).await)
}

async fn refresh(session: Session) -> Json<SessionView> {
    if session.contains("user_id").await {
        session.cycle_id().await;
        session.insert("refreshed", true).await.expect("session insert");
    }

    Json(session_view(&session).await)
}

async fn logout(session: Session) -> NoContent {
    session.destroy().await;
    NoContent
}

let app = RustApi::new()
    .layer(SessionLayer::new(
        MemorySessionStore::new(),
        SessionConfig::new()
            .cookie_name("rustapi_auth")
            .secure(false)
            .ttl(Duration::from_secs(60 * 30)),
    ))
    .route("/auth/login", post(login))
    .route("/auth/me", get(me))
    .route("/auth/refresh", post(refresh))
    .route("/auth/logout", post(logout));
```

A complete runnable version lives in `crates/rustapi-rs/examples/auth_api.rs`.

## How the flow works

1. `SessionLayer` parses the incoming session cookie.
2. The configured store loads the matching `SessionRecord`.
3. The `Session` extractor gives handlers typed access to the record.
4. Handler mutations are persisted after the response is produced.
5. If the session was changed, the middleware emits a new `Set-Cookie` header.
6. `session.destroy().await` deletes the record and clears the cookie.

That means your handlers stay focused on business logic while the middleware handles persistence and cookie management.

## Built-in store options

### In-memory store

Use `MemorySessionStore` for tests, demos, and single-node deployments.

```rust
use rustapi_rs::extras::session::{MemorySessionStore, SessionConfig, SessionLayer};

let layer = SessionLayer::new(
    MemorySessionStore::new(),
    SessionConfig::new(),
);
```

### Redis-backed store

Use `RedisSessionStore` when sessions must survive restarts or be shared across instances.

```rust
use rustapi_rs::extras::session::{RedisSessionStore, SessionConfig, SessionLayer};

let store = RedisSessionStore::from_url(&std::env::var("REDIS_URL")?)?
    .key_prefix("rustapi:session:");

let layer = SessionLayer::new(store, SessionConfig::new());
```

The handler API is identical. Only the store changes.

## Configuration notes

- Keep `cookie_http_only = true` for session cookies.
- Use `secure(true)` in production so cookies are HTTPS-only.
- Use `same_site(SameSite::Lax)` or stricter unless your cross-site flow needs otherwise.
- Rotate the session ID on login and privilege changes with `session.cycle_id().await` to reduce session fixation risk.
- Prefer short TTLs plus rolling expiry for end-user sessions.
- Store only what you need in the session payload. Opaque IDs age better than giant identity blobs.

## Verification

Run the built-in session tests first:

```sh
cargo test -p rustapi-extras --features session
```

Then try the runnable example:

```sh
cargo run -p rustapi-rs --example auth_api --features extras-session
```
