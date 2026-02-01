# JWT Authentication

Authentication is critical for almost every API. RustAPI provides a built-in, production-ready JWT authentication system via the `jwt` feature.

## Dependencies

Enable the `jwt` feature in your `Cargo.toml`:

```toml
[dependencies]
rustapi-rs = { version = "0.1", features = ["jwt"] }
serde = { version = "1", features = ["derive"] }
```

## 1. Define Claims

Define your custom claims struct. It must be serializable and deserializable.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,   // Subject (User ID)
    pub role: String,  // Custom claim: "admin", "user"
    pub exp: usize,    // Required for JWT expiration validation
}
```

## 2. Shared State

To avoid hardcoding secrets in multiple places, we'll store our secret key in the application state.

```rust
#[derive(Clone)]
pub struct AppState {
    pub secret: String,
}
```

## 3. The Handlers

We use the `AuthUser<T>` extractor to protect routes, and `State<T>` to access the secret for signing tokens during login.

```rust
use rustapi_rs::prelude::*;

#[rustapi::get("/profile")]
async fn protected_profile(
    // This handler will only be called if a valid token is present
    AuthUser(claims): AuthUser<Claims>
) -> Json<String> {
    Json(format!("Welcome back, {}! You are a {}.", claims.sub, claims.role))
}

#[rustapi::post("/login")]
async fn login(State(state): State<AppState>) -> Result<Json<String>> {
    // In a real app, validate credentials first!
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() + 3600; // Token expires in 1 hour (3600 seconds)
    
    let claims = Claims {
        sub: "user_123".to_owned(),
        role: "admin".to_owned(),
        exp: expiration as usize,
    };

    // We use the secret from our shared state
    let token = create_token(&claims, &state.secret)?;

    Ok(Json(token))
}
```

## 4. Wiring it Up

Register the `JwtLayer` and the state in your application.

```rust
#[rustapi::main]
async fn main() -> Result<()> {
    // In production, load this from an environment variable!
    let secret = "my_secret_key".to_string();

    let state = AppState {
        secret: secret.clone(),
    };

    // Configure JWT validation with the same secret
    let jwt_layer = JwtLayer::<Claims>::new(secret);

    RustApi::auto()
        .state(state)     // Register the shared state
        .layer(jwt_layer) // Add the middleware
        .run("127.0.0.1:8080")
        .await
}
```

## Bonus: Role-Based Access Control (RBAC)

Since we have the `role` in our claims, we can enforce permissions easily within the handler:

```rust
#[rustapi::get("/admin")]
async fn admin_only(AuthUser(claims): AuthUser<Claims>) -> Result<String, StatusCode> {
    if claims.role != "admin" {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok("Sensitive Admin Data".to_string())
}
```

## How It Works

1. **`JwtLayer` Middleware**: Intercepts requests, looks for `Authorization: Bearer <token>`, validates the signature, and stores the decoded claims in the request extensions.
2. **`AuthUser` Extractor**: Retrieves the claims from the request extensions. If the middleware failed or didn't run, or if the token was missing/invalid, the extractor returns a `401 Unauthorized` error.

This separation allows you to have some public routes (where `JwtLayer` might just pass through) and some protected routes (where `AuthUser` enforces presence). Note that `JwtLayer` by default does *not* reject requests without tokens; it just doesn't attach claims. The *extractor* does the rejection.
