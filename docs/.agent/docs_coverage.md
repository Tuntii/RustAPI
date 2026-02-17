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
| JWT Auth | `docs/cookbook/src/recipes/jwt_auth.md` | `rustapi-extras/src/jwt.rs` (`JwtLayer`) | OK |
| OAuth2 | `docs/cookbook/src/recipes/oauth2_client.md` | `rustapi-extras/src/oauth2.rs` (`OAuth2Client`) | OK |
| Database | `docs/cookbook/src/recipes/db_integration.md` | N/A (Integration pattern) | Needs Update |
| **Ecosystem** | | | |
| WebSockets | `docs/cookbook/src/recipes/websockets.md` | `rustapi-ws/src/lib.rs` (`WebSocketUpgrade`) | OK |
| SSR (View) | `docs/cookbook/src/recipes/server_side_rendering.md` | `rustapi-view/src/lib.rs` (`View`) | OK |
| gRPC | `docs/cookbook/src/recipes/grpc_integration.md` | `rustapi-grpc/src/lib.rs` (`TonicServer`) | OK |
| Jobs | `docs/cookbook/src/recipes/background_jobs.md` | `rustapi-jobs/src/lib.rs` (`Job`) | OK |
| TOON (AI) | `docs/cookbook/src/recipes/ai_integration.md` | `rustapi-toon/src/lib.rs` (`LlmResponse`) | OK |
