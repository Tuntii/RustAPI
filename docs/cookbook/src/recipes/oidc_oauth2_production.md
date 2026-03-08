# OIDC / OAuth2 in Production

This guide turns the basic OAuth2 client into a production-ready login flow.

The short version:

- use `OAuth2Client` to generate the authorization URL,
- store CSRF state and PKCE verifier in a server-side session,
- verify `state` on callback,
- exchange the code for tokens,
- rotate the application session before marking the user as authenticated.

## Prerequisites

Enable both the OAuth2 client and session features on the public facade.

```toml
[dependencies]
rustapi-rs = { version = "0.1.389", features = ["extras-oauth2-client", "extras-session"] }
```

## Configure the provider

Use one of the provider presets when possible.

```rust
use rustapi_rs::extras::oauth2::{OAuth2Client, OAuth2Config};

let config = OAuth2Config::google(
    std::env::var("OAUTH_CLIENT_ID")?,
    std::env::var("OAUTH_CLIENT_SECRET")?,
    std::env::var("OAUTH_REDIRECT_URI")?,
)
.scope("openid")
.scope("email")
.scope("profile");

let client = OAuth2Client::new(config);
```

For non-preset providers, use `OAuth2Config::custom(...)`.

```rust
use rustapi_rs::extras::oauth2::OAuth2Config;

let config = OAuth2Config::custom(
    "https://id.example.com/oauth/authorize",
    "https://id.example.com/oauth/token",
    std::env::var("OAUTH_CLIENT_ID")?,
    std::env::var("OAUTH_CLIENT_SECRET")?,
    std::env::var("OAUTH_REDIRECT_URI")?,
);
```

## Authorization redirect

The authorization handler should generate the provider URL and persist the CSRF + PKCE data in the current session.

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extras::oauth2::OAuth2Client;
use rustapi_rs::extras::session::Session;

async fn oauth_login(State(client): State<OAuth2Client>, session: Session) -> Redirect {
    let auth_request = client.authorization_url();

    session
        .insert("oauth_state", auth_request.csrf_state.as_str())
        .await
        .expect("state should serialize");

    if let Some(pkce) = auth_request.pkce_verifier.as_ref() {
        session
            .insert("oauth_pkce_verifier", pkce.verifier())
            .await
            .expect("pkce verifier should serialize");
    }

    Redirect::to(auth_request.url())
}
```

## Callback handling

The callback handler validates the CSRF state, exchanges the code, and upgrades the application session.

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::extras::oauth2::{CsrfState, OAuth2Client, PkceVerifier};
use rustapi_rs::extras::session::Session;

#[derive(Debug, Deserialize, Schema)]
struct OAuthCallback {
    code: String,
    state: String,
}

async fn oauth_callback(
    State(client): State<OAuth2Client>,
    session: Session,
    Query(callback): Query<OAuthCallback>,
) -> Result<Redirect> {
    let expected_state = session
        .get::<String>("oauth_state")
        .await?
        .ok_or_else(|| ApiError::unauthorized("Missing OAuth state"))?;

    client
        .validate_state(&CsrfState::new(expected_state), &callback.state)
        .map_err(|error| ApiError::unauthorized(error.to_string()))?;

    let pkce_verifier = session
        .get::<String>("oauth_pkce_verifier")
        .await?
        .map(PkceVerifier::new);

    let tokens = client
        .exchange_code(&callback.code, pkce_verifier.as_ref())
        .await
        .map_err(|error| ApiError::unauthorized(error.to_string()))?;

    session.cycle_id().await;
    session.insert("user_id", "provider-subject-here").await?;
    session.insert("refresh_token", tokens.refresh_token()).await?;
    session.remove("oauth_state").await;
    session.remove("oauth_pkce_verifier").await;

    Ok(Redirect::to("/dashboard"))
}
```

## Recommended production shape

### Session strategy

- Keep provider state (`oauth_state`, PKCE verifier, post-login redirect path) in the session, not in query strings.
- Rotate the app session ID after a successful login with `session.cycle_id().await`.
- Prefer `RedisSessionStore` when multiple instances share login traffic.
- Clear bootstrap OAuth keys from the session after the callback succeeds or fails.

### Token handling

- Do not log raw `access_token`, `refresh_token`, or `id_token` values.
- If you only need app authentication, store the provider subject and essential claims instead of the raw access token.
- If you must keep refresh tokens, treat them like secrets: server-side only, never in frontend-readable cookies.
- Call `refresh_token(...)` only from trusted backend paths and overwrite old refresh tokens if the provider rotates them.

### Provider and redirect hygiene

- Use exact HTTPS redirect URIs in production.
- Request the minimum scopes you need.
- Pin timeouts explicitly via `OAuth2Config::timeout(...)` if your provider is slow.
- Prefer issuer/provider presets unless you fully control the custom identity server.

### Identity verification

- OpenID Connect is more than “OAuth + vibes”. Validate the `id_token` with the provider’s JWKs before trusting identity claims.
- Use the provider `userinfo` endpoint only after you decide which claims are authoritative.
- Normalize external identities into your own application user model before starting long-lived sessions.

## Local development

For local work, keep session cookies developer-friendly while still matching production flow structure.

```rust
use rustapi_rs::extras::session::SessionConfig;

let session_config = SessionConfig::new()
    .cookie_name("rustapi_auth")
    .secure(false);
```

That keeps the cookie usable over `http://127.0.0.1:3000` while preserving the same handler code.

## See also

- [OAuth2 Client](oauth2_client.md)
- [Session-Based Authentication](session_auth.md)
