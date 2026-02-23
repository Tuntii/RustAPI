# Security Headers

Security headers are HTTP response headers that help protect your application and its users from various attacks, such as Cross-Site Scripting (XSS), Clickjacking, and MIME sniffing. RustAPI provides a `SecurityHeadersLayer` in `rustapi-extras` to automate this.

## Prerequisites

Add `rustapi-extras` with the `security-headers` feature (or `extras` / `full`).

```toml
[dependencies]
rustapi-extras = { version = "0.1.335", features = ["security-headers"] }
```

## Basic Usage

By default, `SecurityHeadersLayer` applies a balanced set of headers suitable for most applications.

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::security_headers::SecurityHeadersLayer;

#[tokio::main]
async fn main() {
    let app = RustApi::new()
        .layer(SecurityHeadersLayer::new())
        .route("/", get(home));

    // ...
}
```

This adds:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 1; mode=block`
- `Strict-Transport-Security: max-age=31536000; includeSubDomains` (HSTS)
- `Content-Security-Policy: default-src 'self'`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy: geolocation=(), microphone=(), camera=()`

## Strict Mode (Production)

For maximum security, use the `strict()` constructor. This applies stricter policies, including HSTS preload and a locked-down CSP.

```rust
use rustapi_extras::security_headers::SecurityHeadersLayer;

let app = RustApi::new()
    .layer(SecurityHeadersLayer::strict());
```

**⚠️ Warning**: Strict mode sets `HSTS` with `preload`. Only use this if you are sure you want to enforce HTTPS for your domain permanently.

## Custom Configuration

You can customize the layer using the builder pattern.

### Content Security Policy (CSP)

Control which resources the browser is allowed to load.

```rust
use rustapi_extras::security_headers::SecurityHeadersLayer;

let layer = SecurityHeadersLayer::new()
    .csp("default-src 'self'; img-src 'self' https://images.example.com; script-src 'self' https://apis.google.com");
```

### HTTP Strict Transport Security (HSTS)

Enforce HTTPS connections.

```rust
use rustapi_extras::security_headers::{SecurityHeadersLayer, HstsConfig};

let layer = SecurityHeadersLayer::new()
    .hsts(HstsConfig {
        max_age: 63072000, // 2 years
        include_subdomains: true,
        preload: true,
    });
```

To disable HSTS (e.g., for local development on HTTP):

```rust
let layer = SecurityHeadersLayer::new().without_hsts();
```

### X-Frame-Options (Clickjacking Protection)

Control whether your site can be embedded in an iframe.

```rust
use rustapi_extras::security_headers::{SecurityHeadersLayer, XFrameOptions};

// Allow embedding only on the same origin
let layer = SecurityHeadersLayer::new()
    .x_frame_options(XFrameOptions::SameOrigin);
```

### Referrer Policy

Control how much referrer information is sent with requests.

```rust
use rustapi_extras::security_headers::{SecurityHeadersLayer, ReferrerPolicy};

let layer = SecurityHeadersLayer::new()
    .referrer_policy(ReferrerPolicy::NoReferrer);
```

## Testing

You can verify headers using `curl` or browser dev tools.

```bash
curl -I http://localhost:8080/
```

Output:
```http
HTTP/1.1 200 OK
content-type: text/plain
content-length: 2
x-content-type-options: nosniff
x-frame-options: DENY
x-xss-protection: 1; mode=block
strict-transport-security: max-age=31536000; includeSubDomains
content-security-policy: default-src 'self'
referrer-policy: strict-origin-when-cross-origin
permissions-policy: geolocation=(), microphone=(), camera=()
date: Tue, 24 Feb 2026 12:00:00 GMT
```
