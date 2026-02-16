# Docs Coverage Map

| Feature | Documentation | Code Location | Status |
|---------|---------------|---------------|--------|
| **Core** | | | |
| Routing | `concepts/handlers.md` | `rustapi-macros` | OK |
| Extractors | `concepts/handlers.md` | `rustapi-core/src/extract.rs` | OK |
| State | `concepts/handlers.md` | `rustapi-core/src/extract.rs` | OK |
| Validation | `crates/rustapi_validate.md` | `rustapi-validate` | OK |
| **HATEOAS** | | | |
| Pagination | `recipes/pagination.md` | `rustapi-core/src/hateoas.rs` | OK |
| Links | `recipes/pagination.md` | `rustapi-core/src/hateoas.rs` | OK |
| **Extras** | | | |
| Auth (JWT) | `recipes/jwt_auth.md` | `rustapi-extras/src/jwt` | OK |
| Auth (OAuth2) | `recipes/oauth2_client.md` | `rustapi-extras/src/oauth2` | OK |
| Security | `recipes/csrf_protection.md` | `rustapi-extras/src/security` | OK |
| Observability | `crates/rustapi_extras.md` | `rustapi-extras/src/telemetry` | OK |
| Audit Logging | `recipes/audit_logging.md` | `rustapi-extras/src/audit` | OK |
| Middleware (Advanced) | `recipes/advanced_middleware.md` | `rustapi-extras/src/{rate_limit, dedup, cache}` | OK |
| **Jobs** | | | |
| Job Queue (Crate) | `crates/rustapi_jobs.md` | `rustapi-jobs` | OK |
| Background Jobs (Recipe) | `recipes/background_jobs.md` | `rustapi-jobs` | OK |
| **Integrations** | | | |
| gRPC | `recipes/grpc_integration.md` | `rustapi-grpc` | OK |
| SSR | `recipes/server_side_rendering.md` | `rustapi-view` | OK |
| AI / TOON | `recipes/ai_integration.md` | `rustapi-toon` | OK |
| **Learning** | | | |
| Structured Path | `learning/curriculum.md` | N/A | OK |
| **Recipes** | | | |
| File Uploads | `recipes/file_uploads.md` | `rustapi-core` | OK |
| Deployment | `recipes/deployment.md` | `cargo-rustapi` | OK |
| Testing | `recipes/testing.md` | `rustapi-testing` | OK |
