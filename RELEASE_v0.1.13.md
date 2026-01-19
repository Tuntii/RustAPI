# RustAPI v0.1.13 Release Notes 🚀

**Release Date:** January 19, 2026

---

## 🛡️ New Feature: CSRF Protection

RustAPI now includes built-in **Cross-Site Request Forgery (CSRF) protection** using the industry-standard Double-Submit Cookie pattern.

### Quick Start

```rust
use rustapi_rs::prelude::*;
use rustapi_extras::csrf::{CsrfConfig, CsrfLayer, CsrfToken};

#[rustapi_rs::get("/form")]
async fn show_form(token: CsrfToken) -> Html<String> {
    Html(format!(r#"
        <form method="POST" action="/submit">
            <input type="hidden" name="_csrf" value="{}" />
            <button>Submit</button>
        </form>
    "#, token.as_str()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let csrf = CsrfConfig::new()
        .cookie_name("csrf_token")
        .header_name("X-CSRF-Token");

    RustApi::new()
        .layer(CsrfLayer::new(csrf))
        .mount(show_form)
        .run("0.0.0.0:8080")
        .await
}
```

### Features

- ✅ **Double-Submit Cookie Pattern** — Industry standard CSRF protection
- ✅ **CsrfToken Extractor** — Access tokens in handlers
- ✅ **Configurable** — Custom cookie/header names, SameSite policy, secure flags
- ✅ **Frontend Ready** — Works with JavaScript/AJAX and HTML forms
- ✅ **Zero Config Defaults** — Secure by default

---

## 📚 Documentation Updates

- New **CSRF Protection Recipe** in the Cookbook
- Updated **rustapi-extras** documentation with CSRF examples
- Added CSRF to README feature table

---

## 🔧 Bug Fixes

- Fixed clippy lint errors in `rustapi-macros`
- Fixed test imports in `rustapi-extras` CSRF module
- Corrected publish order in `smart_publish.ps1` script

---

## 📦 Installation

```toml
[dependencies]
rustapi-rs = { version = "0.1.13", features = ["csrf"] }
```

Or with all security features:

```toml
rustapi-rs = { version = "0.1.13", features = ["jwt", "cors", "csrf", "rate-limit"] }
```

---

## 🔗 Links

- [Documentation](https://docs.rs/rustapi-rs)
- [GitHub](https://github.com/Tuntii/RustAPI)
- [Cookbook](https://tuntii.github.io/RustAPI/cookbook/)

---

**Full Changelog:** [v0.1.12...v0.1.13](https://github.com/Tuntii/RustAPI/compare/v0.1.12...v0.1.13)
