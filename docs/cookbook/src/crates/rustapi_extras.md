# rustapi-extras: The Toolbox

**Lens**: "The Toolbox"
**Philosophy**: "Batteries included, but swappable."

## Feature Flags

This crate is a collection of production-ready middleware. Everything is behind a feature flag so you don't pay for what you don't use.

| Feature | Component |
|---------|-----------|
| `jwt` | `JwtLayer`, `AuthUser` extractor |
| `cors` | `CorsLayer` |
| `audit` | `AuditStore`, `AuditLogger` |
| `rate-limit` | `RateLimitLayer` |

## Middleware Usage

Middleware wraps your entire API or specific routes.

```rust
let app = RustApi::new()
    .layer(CorsLayer::permissive())
    .layer(CompressionLayer::new())
    .route("/", get(handler));
```

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
