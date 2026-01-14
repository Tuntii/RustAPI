# JWT Authentication

Authentication is critical for almost every API. This recipe demonstrates how to implement JSON Web Token (JWT) authentication using the `jsonwebtoken` crate and RustAPI's extractor pattern.

## Dependencies

Add `jsonwebtoken` and `serde` to your `Cargo.toml`:

```toml
[dependencies]
jsonwebtoken = "9"
serde = { version = "1", features = ["derive"] }
```

## 1. Define Claims

The standard JWT claims. You can add custom fields here (like `role`).

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // Subject (User ID)
    pub exp: usize,   // Expiration time
    pub role: String, // Custom claim: "admin", "user"
}
```

## 2. Configuration State

Store your keys in the application state.

```rust
use std::sync::Arc;
use jsonwebtoken::{EncodingKey, DecodingKey};

#[derive(Clone)]
pub struct AuthState {
    pub encoder: EncodingKey,
    pub decoder: DecodingKey,
}

impl AuthState {
    pub fn new(secret: &str) -> Self {
        Self {
            encoder: EncodingKey::from_secret(secret.as_bytes()),
            decoder: DecodingKey::from_secret(secret.as_bytes()),
        }
    }
}
```

## 3. The `AuthUser` Extractor

This is where the magic happens. We create a custom extractor that:
1. Checks the `Authorization` header.
2. Decodes the token.
3. Validates expiration.
4. Returns the claims or rejects the request.

```rust
use rustapi::prelude::*;
use jsonwebtoken::{decode, Validation, Algorithm};

pub struct AuthUser(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AuthState>> for AuthUser {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut Parts, 
        state: &Arc<AuthState>
    ) -> Result<Self, Self::Rejection> {
        // 1. Get header
        let auth_header = parts.headers.get("Authorization")
            .ok_or((StatusCode::UNAUTHORIZED, Json(json!({"error": "Missing token"}))))?;
            
        let token = auth_header.to_str()
            .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid token format"}))))?
            .strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid token type"}))))?;

        // 2. Decode
        let token_data = decode::<Claims>(
            token, 
            &state.decoder, 
            &Validation::new(Algorithm::HS256)
        ).map_err(|e| (StatusCode::UNAUTHORIZED, Json(json!({"error": e.to_string()}))))?;

        Ok(AuthUser(token_data.claims))
    }
}
```

## 4. Usage in Handlers

Now, securing an endpoint is as simple as adding an argument.

```rust
async fn protected_profile(
    AuthUser(claims): AuthUser
) -> Json<String> {
    Json(format!("Welcome back, {}! You are a {}.", claims.sub, claims.role))
}

async fn login(State(state): State<Arc<AuthState>>) -> Json<String> {
    // In a real app, validate credentials first!
    let claims = Claims {
        sub: "user_123".to_owned(),
        role: "admin".to_owned(),
        exp: 10000000000, // Future timestamp
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(), 
        &claims, 
        &state.encoder
    ).unwrap();

    Json(token)
}
```

## 5. Wiring it Up

```rust
#[tokio::main]
async fn main() {
    let auth_state = Arc::new(AuthState::new("my_secret_key"));

    let app = RustApi::new()
        .route("/login", post(login))
        .route("/profile", get(protected_profile))
        .with_state(auth_state); // Inject state

    RustApi::serve("127.0.0.1:3000", app).await.unwrap();
}
```

## Bonus: Role-Based Access Control (RBAC)

Since we have the `role` in our claims, we can enforce permissions easily.

```rust
async fn admin_only(AuthUser(claims): AuthUser) -> Result<String, StatusCode> {
    if claims.role != "admin" {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok("Sensitive Admin Data".to_string())
}
```
