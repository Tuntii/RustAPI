# CSRF Protection

Cross-Site Request Forgery (CSRF) protection for your RustAPI applications using the **Double-Submit Cookie** pattern.

## What is CSRF?

CSRF is an attack that tricks users into submitting unintended requests. For example, a malicious website could submit a form to your API while users are logged in, performing actions without their consent.

RustAPI's CSRF protection works by:
1. Generating a cryptographic token stored in a cookie
2. Requiring the same token in a request header for state-changing requests
3. Rejecting requests where the cookie and header don't match

## Quick Start

```toml
[dependencies]
rustapi-rs = { version = "0.1", features = ["csrf"] }
```

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::csrf::{CsrfConfig, CsrfLayer, CsrfToken};

#[rustapi_rs::get("/form")]
async fn show_form(token: CsrfToken) -> Html<String> {
    Html(format!(r#"
        <form method="POST" action="/submit">
            <input type="hidden" name="csrf_token" value="{}" />
            <button type="submit">Submit</button>
        </form>
    "#, token.as_str()))
}

#[rustapi_rs::post("/submit")]
async fn handle_submit() -> &'static str {
    // If we get here, CSRF validation passed!
    "Form submitted successfully"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let csrf_config = CsrfConfig::new()
        .cookie_name("csrf_token")
        .header_name("X-CSRF-Token");

    RustApi::new()
        .layer(CsrfLayer::new(csrf_config))
        .mount(show_form)
        .mount(handle_submit)
        .run("127.0.0.1:8080")
        .await
}
```

## Configuration Options

```rust
let config = CsrfConfig::new()
    // Cookie settings
    .cookie_name("csrf_token")      // Default: "csrf_token"
    .cookie_path("/")               // Default: "/"
    .cookie_domain("example.com")   // Default: None (same domain)
    .cookie_secure(true)            // Default: true (HTTPS only)
    .cookie_http_only(false)        // Default: false (JS needs access)
    .cookie_same_site(SameSite::Strict) // Default: Strict
    
    // Token settings
    .header_name("X-CSRF-Token")    // Default: "X-CSRF-Token"
    .token_length(32);              // Default: 32 bytes
```

## How It Works

### Safe Methods (No Validation)

`GET`, `HEAD`, `OPTIONS`, and `TRACE` requests are considered "safe" and don't modify state. The CSRF middleware:

1. ✅ Generates a new token if none exists
2. ✅ Sets the token cookie in the response
3. ✅ **Does NOT validate** the header

### Unsafe Methods (Validation Required)

`POST`, `PUT`, `PATCH`, and `DELETE` requests require CSRF validation:

1. 🔍 Reads the token from the cookie
2. 🔍 Reads the expected token from the header
3. ❌ If missing or mismatched → Returns `403 Forbidden`
4. ✅ If valid → Proceeds to handler

## Frontend Integration

### HTML Forms

For traditional form submissions, include the token as a hidden field:

```html
<form method="POST" action="/api/submit">
    <input type="hidden" name="_csrf" value="{{ csrf_token }}" />
    <!-- form fields -->
    <button type="submit">Submit</button>
</form>
```

### JavaScript / AJAX

For API calls, include the token in the request header:

```javascript
// Read token from cookie
function getCsrfToken() {
    return document.cookie
        .split('; ')
        .find(row => row.startsWith('csrf_token='))
        ?.split('=')[1];
}

// Include in fetch requests
fetch('/api/users', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
        'X-CSRF-Token': getCsrfToken()
    },
    body: JSON.stringify({ name: 'John' })
});
```

### Axios Interceptor

```javascript
import axios from 'axios';

axios.interceptors.request.use(config => {
    if (['post', 'put', 'patch', 'delete'].includes(config.method)) {
        config.headers['X-CSRF-Token'] = getCsrfToken();
    }
    return config;
});
```

## Extracting the Token in Handlers

Use the `CsrfToken` extractor to access the current token in your handlers:

```rust
use rustapi_extras::csrf::CsrfToken;

#[rustapi_rs::get("/api/csrf-token")]
async fn get_csrf_token(token: CsrfToken) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "csrf_token": token.as_str()
    }))
}
```

## Best Practices

### 1. Always Use HTTPS in Production

```rust
let config = CsrfConfig::new()
    .cookie_secure(true);  // Cookie only sent over HTTPS
```

### 2. Use Strict SameSite Policy

```rust
use cookie::SameSite;

let config = CsrfConfig::new()
    .cookie_same_site(SameSite::Strict);  // Most restrictive
```

### 3. Combine with Other Security Measures

```rust
RustApi::new()
    .layer(CsrfLayer::new(csrf_config))
    .layer(SecurityHeadersLayer::strict())  // Add security headers
    .layer(CorsLayer::permissive())         // Configure CORS
```

### 4. Rotate Tokens Periodically

Consider regenerating tokens after sensitive actions:

```rust
#[rustapi_rs::post("/auth/login")]
async fn login(/* ... */) -> impl IntoResponse {
    // After successful login, a new CSRF token will be
    // generated on the next GET request
    // ...
}
```

## Testing CSRF Protection

```rust
use rustapi_testing::{TestClient, TestRequest};

#[tokio::test]
async fn test_csrf_protection() {
    let app = create_app_with_csrf();
    let client = TestClient::new(app);
    
    // GET request should work and set cookie
    let res = client.get("/form").await;
    assert_eq!(res.status(), StatusCode::OK);
    
    let csrf_cookie = res.headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    
    // Extract token value
    let token = csrf_cookie
        .split(';')
        .next()
        .unwrap()
        .split('=')
        .nth(1)
        .unwrap();
    
    // POST without token should fail
    let res = client.post("/submit").await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    
    // POST with correct token should succeed
    let res = client.request(
        TestRequest::post("/submit")
            .header("Cookie", format!("csrf_token={}", token))
            .header("X-CSRF-Token", token)
    ).await;
    assert_eq!(res.status(), StatusCode::OK);
}
```

## Error Handling

When CSRF validation fails, the middleware returns a JSON error response:

```json
{
    "error": {
        "code": "csrf_forbidden",
        "message": "CSRF token validation failed"
    }
}
```

You can customize this by wrapping the layer with your own error handler.

## Security Considerations

| Consideration | Status |
|--------------|--------|
| Token in cookie | ✅ HttpOnly=false (JS needs access) |
| Token validation | ✅ Constant-time comparison |
| SameSite cookie | ✅ Configurable (Strict by default) |
| Secure cookie | ✅ HTTPS-only by default |
| Token entropy | ✅ 32 bytes of cryptographic randomness |

## See Also

- [JWT Authentication](jwt_auth.md) - Token-based authentication
- [Security Headers](../crates/rustapi_extras.md#security-headers) - Additional security layers
- [CORS Configuration](../crates/rustapi_extras.md#cors) - Cross-origin request handling
