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
| `replay` | `ReplayLayer` (Time-Travel Debugging) |
| `timeout` | `TimeoutLayer` |
| `guard` | `PermissionGuard` |
| `sanitization` | Input sanitization utilities |

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
rustapi-extras = { version = "0.1.300", features = ["insight"] }
```

### Setup

```rust
use rustapi_extras::insight::{InsightLayer, InMemoryInsightStore, InsightConfig};
use std::sync::Arc;

let store = Arc::new(InMemoryInsightStore::new());
let config = InsightConfig::default();

let app = RustApi::new()
    .layer(InsightLayer::new(config, store.clone()));
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

## Observability

The `otel` and `structured-logging` features bring enterprise-grade observability.

### OpenTelemetry

```rust
use rustapi_extras::otel::{OtelLayer, OtelConfig};

let config = OtelConfig::default().service_name("my-service");
let app = RustApi::new()
    .layer(OtelLayer::new(config));
```

### Structured Logging

Emit logs as JSON for aggregators like Datadog or Splunk. This is different from request logging; it formats your application logs.

```rust
use rustapi_extras::structured_logging::{StructuredLoggingLayer, JsonFormatter};

let app = RustApi::new()
    .layer(StructuredLoggingLayer::new(JsonFormatter::default()));
```

## Advanced Security

### OAuth2 Client

The `oauth2-client` feature provides a complete client implementation.

```rust
use rustapi_extras::oauth2::{OAuth2Client, OAuth2Config, Provider};

let config = OAuth2Config::new(
    Provider::Google,
    "client_id",
    "client_secret",
    "http://localhost:8080/callback"
);
let client = OAuth2Client::new(config);
```

### Security Headers

Add standard security headers (HSTS, X-Frame-Options, etc.).

```rust
use rustapi_extras::security_headers::SecurityHeadersLayer;

let app = RustApi::new()
    .layer(SecurityHeadersLayer::default());
```

### API Keys

Simple API Key authentication strategy.

```rust
use rustapi_extras::api_key::ApiKeyLayer;

let app = RustApi::new()
    .layer(ApiKeyLayer::new("my-secret-key"));
```

### Permission Guards

The `guard` feature provides role-based access control (RBAC) helpers.

```rust
use rustapi_extras::guard::PermissionGuard;

// Only allows users with "admin" role
#[rustapi_rs::get("/admin")]
async fn admin_panel(
    _guard: PermissionGuard
) -> &'static str {
    "Welcome Admin"
}
```

### Input Sanitization

The `sanitization` feature helps prevent XSS by cleaning user input.

```rust
use rustapi_extras::sanitization::sanitize_html;

let safe_html = sanitize_html("<script>alert(1)</script>Hello");
// Result: "Hello"
```

## Resilience

### Circuit Breaker

Prevent cascading failures by stopping requests to failing upstreams.

```rust
use rustapi_extras::circuit_breaker::CircuitBreakerLayer;

let app = RustApi::new()
    .layer(CircuitBreakerLayer::new());
```

### Retry

Automatically retry failed requests with backoff.

```rust
use rustapi_extras::retry::RetryLayer;

let app = RustApi::new()
    .layer(RetryLayer::default());
```

### Timeout

Ensure requests don't hang indefinitely.

```rust
use rustapi_extras::timeout::TimeoutLayer;
use std::time::Duration;

let app = RustApi::new()
    .layer(TimeoutLayer::new(Duration::from_secs(30)));
```

## Optimization

### Caching

Cache responses based on headers or path.

```rust
use rustapi_extras::cache::CacheLayer;

let app = RustApi::new()
    .layer(CacheLayer::new());
```

### Request Deduplication

Prevent duplicate requests (e.g., from double clicks) from processing twice.

```rust
use rustapi_extras::dedup::DedupLayer;

let app = RustApi::new()
    .layer(DedupLayer::new());
```

## Debugging

### Time-Travel Debugging (Replay)

The `replay` feature allows you to record production traffic and replay it locally for debugging.

See the [Time-Travel Debugging Recipe](../recipes/replay.md) for full details.

```rust
use rustapi_extras::replay::{ReplayLayer, ReplayConfig, InMemoryReplayStore};

let replay_config = ReplayConfig::default();
let store = InMemoryReplayStore::new(1_000);

let app = RustApi::new()
    .layer(ReplayLayer::new(replay_config).with_store(store));
```
