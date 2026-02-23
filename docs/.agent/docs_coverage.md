# Documentation Coverage Map

Last Updated: 2026-02-24
Version: 0.1.335

## Crates Coverage

| Crate | Main Docs | Recipes | Status | Notes |
|-------|-----------|---------|--------|-------|
| `rustapi-rs` | `GETTING_STARTED.md`, `README.md` | Many | ✅ OK | Main entry point |
| `rustapi-core` | `crates/rustapi_core.md` | `file_uploads.md`, `custom_middleware.md` | ✅ OK | Core features covered |
| `rustapi-extras` | `crates/rustapi_extras.md` | See Features table below | ✅ OK | Extensive feature set |
| `rustapi-jobs` | `crates/rustapi_jobs.md` | `background_jobs.md` | ✅ OK | |
| `rustapi-validate` | `crates/rustapi_validation.md` | `learning/curriculum.md` (Module 5) | ✅ OK | |
| `rustapi-openapi` | `crates/rustapi_openapi.md` | `openapi_refs.md` | ✅ OK | |
| `rustapi-toon` | `crates/rustapi_toon.md` | `ai_integration.md` | ✅ OK | |
| `rustapi-ws` | `crates/rustapi_ws.md` | `websockets.md` | ✅ OK | |
| `rustapi-view` | `crates/rustapi_view.md` | `server_side_rendering.md` | ✅ OK | |
| `rustapi-testing` | `crates/rustapi_testing.md` | `testing.md` | ✅ OK | |
| `rustapi-grpc` | `crates/rustapi_grpc.md` | `grpc_integration.md` | ✅ OK | |
| `rustapi-macros` | `crates/rustapi_macros.md` | N/A | ✅ OK | Internal details |
| `cargo-rustapi` | `crates/cargo_rustapi.md` | N/A | ✅ OK | CLI tool |

## Features Coverage (rustapi-extras)

| Feature | Docs Location | Status | Notes |
|---------|---------------|--------|-------|
| `jwt` | `recipes/jwt_auth.md` | ✅ OK | |
| `cors` | `crates/rustapi_extras.md` | ⚠️ Basic | Could use a dedicated recipe |
| `rate-limit` | `recipes/advanced_middleware.md` | ✅ OK | |
| `csrf` | `recipes/csrf_protection.md` | ✅ OK | |
| `config` | `crates/rustapi_extras.md` | ⚠️ Basic | Env var loading |
| `cookies` | `crates/rustapi_extras.md` | ⚠️ Missing | Only brief mention |
| `sqlx` | `recipes/db_integration.md` | ✅ OK | |
| `insight` | `crates/rustapi_extras.md` | ✅ OK | Detailed section in crate docs |
| `timeout` | `recipes/resilience.md` | ✅ OK | |
| `guard` | `crates/rustapi_extras.md` | ⚠️ Basic | Permission guards |
| `logging` | `recipes/audit_logging.md` | ✅ OK | Audit logging |
| `circuit-breaker` | `recipes/resilience.md` | ✅ OK | |
| `retry` | `recipes/resilience.md` | ✅ OK | |
| `dedup` | `recipes/advanced_middleware.md` | ✅ OK | |
| `sanitization` | `crates/rustapi_extras.md` | ⚠️ Basic | |
| `security-headers` | `recipes/security_headers.md` | ✅ OK | Recipe added 2026-02-24 |
| `api-key` | `crates/rustapi_extras.md` | ⚠️ Basic | |
| `cache` | `recipes/advanced_middleware.md` | ✅ OK | |
| `otel` | `crates/rustapi_extras.md` | ⚠️ Basic | Could use more on integration |
| `structured-logging`| `crates/rustapi_extras.md` | ⚠️ Basic | |
| `oauth2-client` | `recipes/oauth2_client.md` | ✅ OK | |
| `audit` | `recipes/audit_logging.md` | ✅ OK | |
| `replay` | `recipes/replay.md` | ✅ OK | |

## Missing / Needs Improvement

- **File Uploads**: Needs streaming example (or explicit warning about lack thereof).
- **Validation**: Manual validation for multipart fields.
- **Reverse Proxy**: Explicit guide on configuring `RateLimitLayer` for proxies.
- **Observability**: A full recipe combining `otel`, `structured-logging`, and `insight` would be valuable.
- **Security**: Dedicated `Security Headers` recipe needed (Planned).

## Plan for 2026-02-24 Run

1.  Update `file_uploads.md` with validation & streaming caveats.
2.  Update `advanced_middleware.md` with Reverse Proxy info.
3.  Add `security_headers.md` recipe.
4.  Enhance Learning Path Module 5.
