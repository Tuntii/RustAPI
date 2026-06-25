# OAuth2 Client Integration

Integrating with third-party identity providers (like Google, GitHub) is a common requirement for modern applications. RustAPI exposes the OAuth2 client through the public `rustapi-rs` facade.

This recipe demonstrates how to set up an OAuth2 flow.

## Prerequisites

Enable the canonical facade feature in `rustapi-rs`.

```toml
[dependencies]
rustapi-rs = { version = "0.1.389", features = ["extras-oauth2-client"] }
```

## Basic Configuration

You can use presets for popular providers or configure a custom one.

```rust
use rustapi_rs::extras::oauth2::OAuth2Config;

// Using a preset (Google)
let config = OAuth2Config::google(
    "your-client-id",
    "your-client-secret",
    "https://your-app.com/auth/callback/google"
);

// Or custom provider
let custom_config = OAuth2Config::custom(
    "https://auth.example.com/authorize",
    "https://auth.example.com/token",
    "client-id",
    "client-secret",
    "https://your-app.com/callback",
);
```

## The Authorization Flow

1.  **Redirect User**: Generate an authorization URL and redirect the user.
2.  **Handle Callback**: Exchange the authorization code for an access token.

### Step 1: Redirect User

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extras::oauth2::OAuth2Client;
use rustapi_rs::extras::session::Session;

async fn login(State(client): State<OAuth2Client>, session: Session) -> Redirect {
    // Generate URL with CSRF protection and PKCE
    let auth_request = client.authorization_url();

    session.insert("oauth_state", auth_request.csrf_state.as_str()).await.expect("state should serialize");
    if let Some(pkce) = auth_request.pkce_verifier.as_ref() {
        session.insert("oauth_pkce_verifier", pkce.verifier()).await.expect("pkce should serialize");
    }

    // Redirect user
    Redirect::to(auth_request.url().as_str())
}
```

### Step 2: Handle Callback

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extras::oauth2::{CsrfState, OAuth2Client, PkceVerifier};
use rustapi_rs::extras::session::Session;

#[derive(Deserialize)]
struct AuthCallback {
    code: String,
    state: String, // CSRF token
}

async fn callback(
    State(client): State<OAuth2Client>,
    session: Session,
    Query(params): Query<AuthCallback>,
) -> impl IntoResponse {
    let expected_state = session.get::<String>("oauth_state").await.unwrap().unwrap();
    client
        .validate_state(&CsrfState::new(expected_state), &params.state)
        .expect("invalid oauth state");

    let pkce_verifier = session
        .get::<String>("oauth_pkce_verifier")
        .await
        .unwrap()
        .map(PkceVerifier::new);

    // 2. Exchange code for token
    let token_response = client.exchange_code(&params.code, pkce_verifier.as_ref()).await;

    match token_response {
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
3.  **Session Storage**: Store the CSRF state and PKCE verifier in a secure server-side session. Pair `extras-oauth2-client` with `extras-session` for the cleanest flow.
4.  **Secure Storage**: Store tokens securely (e.g., encrypted cookies, secure session storage). Never expose access tokens in URLs or logs.
4.  **HTTPS**: OAuth2 requires HTTPS callbacks in production.

For a production-focused checklist, redirect strategy, and session integration guidance, continue with [OIDC & OAuth2 in Production](oidc_oauth2_production.md).
