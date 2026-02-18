# Docs Coverage Map

| Feature Area | Documentation Page | Source Code (Key Crates) | Status | Notes |
|--------------|-------------------|--------------------------|--------|-------|
| **Core Routing** | `concepts/handlers.md` | `rustapi-core` | OK | |
| **Extractors** | `concepts/handlers.md` | `rustapi-core` | OK | `Body`, `Json`, `Path`, `Query`, `State` |
| **Validation** | `crates/rustapi_validate.md` | `rustapi-validate` | OK | `#[derive(Validate)]`, `ValidatedJson` |
| **OpenAPI** | `crates/rustapi_openapi.md` | `rustapi-openapi` | OK | `#[derive(Schema)]` |
| **WebSocket** | `recipes/websockets.md` | `rustapi-ws` | OK | Upgrade handling, broadcasting |
| **Database** | `recipes/db_integration.md` | `rustapi-core` | OK | Added pooling, transactions, testing |
| **File Uploads** | `recipes/file_uploads.md` | `rustapi-core` | OK | Fixed code example |
| **Compression** | `recipes/compression.md` | `rustapi-core` | OK | Recipe created |
| **Authentication** | `recipes/jwt_auth.md`, `recipes/oauth2_client.md` | `rustapi-extras` | OK | JWT, OAuth2 |
| **Observability** | `crates/rustapi_extras.md`, `recipes/audit_logging.md` | `rustapi-extras` | OK | Tracing, Metrics, Audit |
| **Resilience** | `recipes/resilience.md` | `rustapi-extras` | OK | Circuit Breaker, Retry, Timeout |
| **Background Jobs** | `recipes/background_jobs.md` | `rustapi-jobs` | OK | Job queue, workers |
| **Testing** | `concepts/testing.md` | `rustapi-testing` | OK | `TestClient`, Mocking |
| **SSR** | `recipes/server_side_rendering.md` | `rustapi-view` | OK | Tera integration |
| **gRPC** | `recipes/grpc_integration.md` | `rustapi-grpc` | OK | Tonic integration |
| **AI / TOON** | `recipes/ai_integration.md` | `rustapi-toon` | OK | TOON format |
| **HTTP/3** | `recipes/http3_quic.md` | `rustapi-core` | OK | QUIC support |
| **OpenAPI Refs** | `recipes/openapi_refs.md` | `rustapi-openapi` | OK | Modular schemas, Refs created |
