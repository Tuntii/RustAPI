# RustAPI v0.1.15: Deployment Tooling, HTTP/3 & Validation i18n

**Date:** 2026-01-23

## 🚀 Key Highlights

*   **1-Command Deployment**: `cargo rustapi deploy` now supports effortless deployment to Fly.io, Railway, and Shuttle.rs. No manual Dockerfile wrangling required!
*   **HTTP/3 (QUIC) Support**: The core engine now supports HTTP/3 powered by `quinn` and `h3`. Enable the `http3` feature for lower latency and better performance on unreliable networks.
*   **International Validation**: Native i18n support in `rustapi-validate`. Error messages can now be localized (defaults to EN, supports TR).
*   **Client Generator**: Generate type-safe API clients for Rust, Python, and TypeScript directly from your code.

---

## 📋 Full Changelog

### Added
- **Deployment Tooling (cargo-rustapi)**: Added `deploy` command supporting Docker, Fly.io, Railway, and Shuttle.rs with config generation. Added OpenAPI client generation for Rust, TypeScript, and Python. Updated dependencies for YAML support and remote specs.
- **HTTP/3 (QUIC) Support (rustapi-core)**: Added HTTP/3 infrastructure using Quinn + h3 stack. Supports self-signed certs (dev) and dual-stack execution. Added `http3` and `http3-dev` features.
- **HATEOAS & ReDoc Improvements**: Added HATEOAS module to `rustapi-core` (HAL-style links, resource wrappers, pagination). Refactored ReDoc HTML generation in `rustapi-openapi` with exposed configuration.
- **Validation i18n & New Capabilities**: Added i18n support (rust-i18n) with EN/TR locales in `rustapi-validate`. Refactored rule messages to use message keys. Added custom async validation support (parsing, macros, tests). Removed `validator` crate dependency.

### Changed
- **Unified Response Body**: Refactored `rustapi-core` to use a unified `Body`/`ResponseBody` abstraction for full and streaming responses. Standardized middleware and cache layers.
- **Streaming Behavior**: Clarified flow behavior for streaming responses (e.g., explicit `Transfer-Encoding: chunked`).
- **Server Lifecycle**: Added graceful shutdown signal method for better lifecycle control.
- **OpenAPI Path Params**: Added support for custom schema type overrides for path parameters via `#[rustapi::param]` and `.param()`.

### Fixed
- **Validation Groups**: Fixed logic for default group application and context boundaries.
- **Circuit Breaker**: Fixed syntax error in `circuit_breaker.rs`.
- **OpenAPI**: Improved param schema/type handling; fixed UUID path param integer display bug (#55).

### Documentation
- **Traits**: Reorganized trait documentation examples.
- **Cookbook**: Added deployment and HTTP/3 documents.
- **General**: Updated crate pages and README contact info.

### CI / Tooling
- **Coverage**: Added GitHub Actions job for coverage generation with `cargo-tarpaulin`.

### Chores
- **Refactoring**: Moved modules in `rustapi-extras` to subdirectories. Cleaned up unused imports and small refactors across the workspace.
- **Versioning**: Bumped workspace versions to 0.1.15.
