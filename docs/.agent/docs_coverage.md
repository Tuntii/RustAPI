# Docs Coverage Map

| Feature Area | Documentation Page | Source Code (Key Symbols) | Status |
|--------------|-------------------|--------------------------|--------|
| **Core** | | | |
| Routing | `docs/cookbook/src/concepts/routing.md` | `rustapi-core/src/router.rs` (`Router`) | OK |
| Handlers | `docs/cookbook/src/concepts/handlers.md` | `rustapi-core/src/handler.rs` (`Handler`) | OK |
| Extractors | `docs/cookbook/src/concepts/extractors.md` | `rustapi-core/src/extract.rs` (`FromRequest`) | OK |
| Middleware | `docs/cookbook/src/recipes/custom_middleware.md` | `rustapi-core/src/middleware/mod.rs` (`MiddlewareLayer`) | OK |
| State | `docs/cookbook/src/concepts/state.md` | `rustapi-core/src/extract.rs` (`State`) | OK |
| Error Handling | `docs/cookbook/src/concepts/errors.md` | `rustapi-core/src/error.rs` (`ApiError`) | OK |
| HTTP/3 (QUIC) | `docs/cookbook/src/recipes/http3_quic.md` | `rustapi-core/src/http3.rs` (`Http3Server`) | OK |
| File Uploads | `docs/cookbook/src/recipes/file_uploads.md` | `rustapi-core/src/multipart.rs` (`Multipart`) | OK |
| Compression | `docs/cookbook/src/recipes/compression.md` | `rustapi-core/src/middleware/compression.rs` (`CompressionLayer`) | OK |
| **OpenAPI** | | | |
| Schema Derivation | `docs/cookbook/src/crates/rustapi_openapi.md` | `rustapi-macros/src/derive_schema.rs` (`#[derive(Schema)]`) | OK |
| References ($ref) | `docs/cookbook/src/recipes/openapi_refs.md` | `rustapi-openapi/src/schema.rs` (`SchemaRef`) | OK |
| **Validation** | | | |
| Sync Validation | `docs/cookbook/src/crates/rustapi_validate.md` | `rustapi-validate/src/lib.rs` (`Validate`) | OK |
| Async Validation | `docs/cookbook/src/crates/rustapi_validate.md` | `rustapi-validate/src/v2/mod.rs` (`AsyncValidate`) | OK |
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
| WebSockets | `recipes/websockets.md` | `rustapi-ws` | Updated |
| **Learning** | | | |
| Structured Path | `learning/curriculum.md` | N/A | Updated (Mini Projects) |
| **Recipes** | | | |
| File Uploads | `recipes/file_uploads.md` | `rustapi-core` | Updated (Buffered) |
| Deployment | `recipes/deployment.md` | `cargo-rustapi` | OK |
| Testing | `recipes/testing.md` | `rustapi-testing` | OK |
