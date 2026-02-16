# OAuth2 Client Integration

Integrating with third-party identity providers (like Google, GitHub) is a common requirement for modern applications. RustAPI provides a streamlined OAuth2 client in `rustapi-extras`.

This recipe demonstrates how to set up an OAuth2 flow.

## Prerequisites

Add `rustapi-extras` with the `oauth2-client` feature to your `Cargo.toml`.

```toml
[dependencies]
rustapi-extras = { version = "0.1.335", features = ["oauth2-client"] }
```

## Basic Configuration

You can use presets for popular providers or configure a custom one.

```rust
use rustapi_extras::oauth2::{OAuth2Config, Provider};

// Using a preset (Google)
let config = OAuth2Config::google(
    "your-client-id",
    "your-client-secret",
    "https://your-app.com/auth/callback/google"
);

// Or custom provider
let custom_config = OAuth2Config::new(
    "client-id",
    "client-secret",
    "https://auth.example.com/authorize",
    "https://auth.example.com/token",
    "https://your-app.com/callback"
);
```

## The Authorization Flow

1.  **Redirect User**: Generate an authorization URL and redirect the user.
2.  **Handle Callback**: Exchange the authorization code for an access token.

### Step 1: Redirect User

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::oauth2::{OAuth2Client, OAuth2Config};

async fn login(client: State<OAuth2Client>) -> impl IntoResponse {
    // Generate URL with CSRF protection and PKCE
    let auth_request = client.authorization_url();

    // Store CSRF token and PKCE verifier in session (or cookie)
    // In a real app, use secure, http-only cookies
    // session.insert("csrf_token", auth_request.csrf_state.secret());
    // session.insert("pkce_verifier", auth_request.pkce_verifier.secret());

    // Redirect user
    Redirect::to(auth_request.url().as_str())
}
```

### Step 2: Handle Callback

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::oauth2::{OAuth2Client, OAuth2Config};

#[derive(Deserialize)]
struct AuthCallback {
    code: String,
    state: String, // CSRF token
}

async fn callback(
    Query(params): Query<AuthCallback>,
    client: State<OAuth2Client>,
    // session: Session, // Assuming session management
) -> impl IntoResponse {
    // 1. Verify CSRF token from session matches params.state

    // 2. Exchange code for token
    // let pkce_verifier = session.get("pkce_verifier").unwrap();

    match client.exchange_code(&params.code, /* pkce_verifier */).await {
        Ok(token_response) => {
            // Success! You have an access token.
            // Use it to fetch user info or store it.
            println!("Access Token: {}", token_response.access_token());

            // Redirect to dashboard or home
            Redirect::to("/dashboard")
        }
        Err(e) => {
            // Handle error (e.g., invalid code)
            (StatusCode::BAD_REQUEST, format!("Auth failed: {}", e)).into_response()
        }
    }
}
```

## User Information

Once you have an access token, you can fetch user details. Most providers offer a `/userinfo` endpoint.

```rust
// Example using reqwest (feature required)
async fn get_user_info(token: &str) -> Result<serde_json::Value, reqwest::Error> {
    let client = reqwest::Client::new();
    client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(token)
        .send()
        .await?
        .json()
        .await
}
```

## Best Practices

1.  **State Parameter**: Always use the `state` parameter to prevent CSRF attacks. RustAPI's `authorization_url()` generates one for you.
2.  **PKCE**: Proof Key for Code Exchange (PKCE) is recommended for all OAuth2 flows, especially for public clients. RustAPI handles PKCE generation.
3.  **Secure Storage**: Store tokens securely (e.g., encrypted cookies, secure session storage). Never expose access tokens in URLs or logs.
4.  **HTTPS**: OAuth2 requires HTTPS callbacks in production.
