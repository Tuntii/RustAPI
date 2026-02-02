# rustapi-extras: The Toolbox

**Lens**: "The Toolbox"
**Philosophy**: "Batteries included, but swappable."

## Feature Flags

This crate is a collection of production-ready middleware. Everything is behind a feature flag so you don't pay for what you don't use.

| Feature | Component |
|---------|-----------|
| `jwt` | `JwtLayer`, `AuthUser` extractor |
| `cors` | `CorsLayer` |
| `csrf` | `CsrfLayer`, `CsrfToken` extractor |
| `audit` | `AuditStore`, `AuditLogger` |
| `insight` | `InsightLayer`, `InsightStore` |
| `rate-limit` | `RateLimitLayer` |

## Middleware Usage

Middleware wraps your entire API or specific routes.

```rust
let app = RustApi::new()
    .layer(CorsLayer::permissive())
    .layer(CompressionLayer::new())
    .route("/", get(handler));
```

## CSRF Protection

Cross-Site Request Forgery protection using the Double-Submit Cookie pattern.

```rust
use rustapi_extras::csrf::{CsrfConfig, CsrfLayer, CsrfToken};

// Configure CSRF middleware
let csrf_config = CsrfConfig::new()
    .cookie_name("csrf_token")
    .header_name("X-CSRF-Token")
    .cookie_secure(true);        // HTTPS only

let app = RustApi::new()
    .layer(CsrfLayer::new(csrf_config))
    .route("/form", get(show_form))
    .route("/submit", post(handle_submit));
```

### Extracting the Token

Use the `CsrfToken` extractor to access the token in handlers:

```rust
#[rustapi_rs::get("/form")]
async fn show_form(token: CsrfToken) -> Html<String> {
    Html(format!(r#"
        <input type="hidden" name="_csrf" value="{}" />
    "#, token.as_str()))
}
```

### How It Works

1. **Safe methods** (`GET`, `HEAD`) generate and set the token cookie
2. **Unsafe methods** (`POST`, `PUT`, `DELETE`) require the token in the `X-CSRF-Token` header
3. If header doesn't match cookie → `403 Forbidden`

See [CSRF Protection Recipe](../recipes/csrf_protection.md) for a complete guide.

## Audit Logging

For enterprise compliance (GDPR/SOC2), the `audit` feature provides a structured way to record sensitive actions.

```rust
async fn delete_user(
    AuthUser(user): AuthUser,
    State(audit): State<AuditLogger>
) {
    audit.log(AuditEvent::new("user.deleted")
        .actor(user.id)
        .target("user_123")
    );
}
```

## Traffic Insight

The `insight` feature provides powerful real-time traffic analysis and debugging capabilities without external dependencies. It is designed to be low-overhead and privacy-conscious.

```toml
[dependencies]
rustapi-extras = { version = "0.1.233", features = ["insight"] }
```

### Setup

```rust
use rustapi_extras::insight::{InsightLayer, InMemoryInsightStore, InsightConfig};
use std::sync::Arc;

let store = Arc::new(InMemoryInsightStore::new(InMemoryInsightStore::default_capacity()));
let config = InsightConfig::default();

let app = RustApi::new()
    .state(store.clone())
    .layer(InsightLayer::with_config(config).with_store(store.clone()));
```

### Accessing Data

You can inspect the collected data (e.g., via an admin dashboard):

```rust
#[rustapi_rs::get("/admin/insights")]
async fn get_insights(State(store): State<Arc<InMemoryInsightStore>>) -> Json<InsightStats> {
    // Returns aggregated stats like req/sec, error rates, p99 latency
    Json(store.get_stats().await)
}
```

The `InsightStore` trait allows you to implement custom backends (e.g., ClickHouse or Elasticsearch) if you need long-term retention.
