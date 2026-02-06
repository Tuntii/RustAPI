# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.275] - 2026-02-06

### Added
- **Replay (Time-Travel Debugging)**: Complete time-travel debugging system for recording and replaying HTTP requests
  - **rustapi-core**: Pure types and traits (ReplayConfig, compute_diff, ReplayEntry, ReplayMeta, redaction, ReplayStore trait, truncation)
  - **rustapi-extras**: Production-ready implementation
    - `ReplayLayer` middleware for automatic request/response recording
    - `InMemoryReplayStore` and `FileSystemReplayStore` implementations
    - Admin HTTP routes for listing, replaying, and diffing entries
    - `ReplayClient` for programmatic replay testing
    - Authentication with bearer token
    - `RetentionJob` for automatic cleanup of expired entries
  - **cargo-rustapi**: CLI commands for replay management (requires `replay` feature)
    - Install with: `cargo install cargo-rustapi --features replay`
    - `cargo rustapi replay list` - List recorded entries
    - `cargo rustapi replay show <id>` - Show entry details
    - `cargo rustapi replay run <id> --target <url>` - Replay request
    - `cargo rustapi replay diff <id> --target <url>` - Compare responses
    - `cargo rustapi replay delete <id>` - Delete entry
  - Security features: disabled by default, admin token required, sensitive header/body redaction, configurable TTL
  - Cookbook recipe with comprehensive examples and security guidelines

### Fixed
- Fixed broken intra-doc link to `ReplayLayer` in rustapi-core replay module documentation

### Removed
- Removed unused `check_diff.py` script from repository root

## [0.1.202] - 2026-01-26

### Performance - 12x Improvement 🚀

This release delivers a **12x performance improvement**, bringing RustAPI from ~8K req/s to **~92K req/s**.

#### Benchmark Results

| Framework | Requests/sec | Latency (avg) |
|-----------|-------------|---------------|
| **RustAPI** | ~92,000 | ~1.1ms |
| Actix-web 4 | ~105,000 | ~0.95ms |
| Axum | ~100,000 | ~1.0ms |

*Tested with `hey -n 100000 -c 100` on Windows 11, Ryzen 9 5900X*

### Added
- **Ultra-Fast Path**: New routing path that bypasses both middleware AND interceptors for maximum performance
- **simd-json Serialization**: Extended simd-json support from parsing-only to full serialization with `to_vec` and `to_vec_with_capacity`

### Changed
- **TCP_NODELAY**: Disabled Nagle's algorithm for lower latency
- **Pipeline Flush**: Enabled HTTP/1.1 pipeline flushing for better throughput
- **ConnectionService**: Reduced Arc cloning overhead per connection
- **HandleRequestFuture**: Custom future implementation for request handling

### Fixed
- Removed unused static variables from bench_server

### Documentation
- Updated README.md with accurate benchmark numbers
- Removed inflated performance claims
- Added TechEmpower-based comparison data
- Created [BEAT_ACTIX_ROADMAP.md](memories/BEAT_ACTIX_ROADMAP.md) for future optimizations

## [0.1.15] - 2026-01-23

### Added
- **Deployment Tooling (cargo-rustapi)**: Added `deploy` command supporting Docker, Fly.io, Railway, and Shuttle.rs with config generation. Added OpenAPI client generation for Rust, TypeScript, and Python. Updated dependencies for YAML support and remote specs.
- **HTTP/3 (QUIC) Support (rustapi-core)**: Added HTTP/3 infrastructure using Quinn + h3 stack. Supports self-signed certs (dev) and dual-stack execution. Added `http3` and `http3-dev` features.
- **HATEOAS & ReDoc Improvements**: Added HATEOAS module to `rustapi-core` (HAL-style links, resource wrappers, pagination). Refactored ReDoc HTML generation in `rustapi-openapi` with exposed configuration.
- **Validation i18n & New Capabilities**: Added i18n support (rust-i18n) with EN/TR locales in `rustapi-validate`. Refactored rule messages to use message keys. Added custom async validation support (parsing, macros, tests). Removed `validator` crate dependency.

### Changed
- **Unified Response Body**: Refactored `rustapi-core` to use a unified `Body`/`ResponseBody` abstraction for full and streaming responses. Standardized middleware and cache layers.
- **Streaming Behavior**: Clarified flow behavior for streaming responses (e.g., explicit `Transfer-Encoding: chunked`).
- **Server Lifecycle**: Added graceful shutdown signal method for better lifecycle control.
- **OpenAPI Path Params**: Added support for custom schema type overrides for path parameters via `#[rustapi_rs::param]` and `.param()`.

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

## [0.1.11] - 2026-01-14

### Fixed
- **WebSocket**: Fixed `ManualUpgrade` error by properly draining request body before upgrade in `rustapi-ws`.
- **WebSocket**: Enabled `http1::Builder::with_upgrades()` in `rustapi-core` server for Hyper 1.0 compatibility.
- **Examples**: Fixed compilation issues in `phase11-demo` and `graphql-api`.
- **Examples**: Updated `websocket` example with better debug logging.

## [0.1.10] - 2026-01-14

### Documentation
- Updated READMEs for all crates with comprehensive examples and better structure.
- Improved `Cargo.toml` descriptions and documentation links.

## [0.1.9] - 2026-01-14

### Added

#### Performance (big-performance branch)
- **`simd-json` feature**: 2-4x faster JSON parsing when enabled
- **Stack-optimized `PathParams`**: Using `SmallVec<[_; 4]>` for fewer allocations
- **Conditional tracing**: Logging gated behind `tracing` feature for 10-20% less overhead
- **Streaming request body**: Support for large/unbuffered bodies without full memory buffering

#### New Crates
- **`rustapi-jobs`**: Background job processing
  - In-memory, Redis, and Postgres backends
  - Job queue with retry logic and exponential backoff
  - Dead letter queue for failed jobs
  - Scheduled and delayed execution
- **`rustapi-testing`**: Test utilities
  - `TestServer` for spawning test instances
  - `Matcher` for response body/header matching
  - `Expectation` builder for fluent assertions

#### Security & Compliance
- **Audit Logging System** in `rustapi-extras`
  - GDPR and SOC2 compliance support
  - In-memory and file-based audit stores
  - Event/query types with store trait

#### CLI Improvements (`cargo-rustapi`)
- `cargo rustapi watch` — Auto-reload on file changes
- `cargo rustapi add` — Add dependencies or features
- `cargo rustapi doctor` — Check environment health

#### Testing
- **Property-based tests** with `proptest`:
  - Streaming memory bounds validation
  - Audit event completeness
  - CSRF token lifecycle
  - OAuth2 token exchange round-trips
  - OpenTelemetry trace context propagation
  - Structured logging format compliance

#### New Examples
- `event-sourcing` — CQRS/Event Sourcing demo
- `microservices-advanced` — Multi-binary with Docker + service discovery
- `serverless-lambda` — AWS Lambda integration

### Fixed
- Fixed async handling and error mapping in file writes
- Fixed Redis `zadd` call in job backend
- Enabled `r2d2` feature for diesel, clarified error types
- Removed unused imports across crates

## [0.1.8] - 2026-01-10

### Added
- **CORS middleware**: `CorsLayer` with full `MiddlewareLayer` trait implementation
  - Support for `CorsLayer::permissive()` and custom configuration
  - Proper preflight request handling
  - Origin validation and credential support

### Fixed
- Fixed missing `MiddlewareLayer` implementation for `CorsLayer`
- Fixed CI build issues with GitHub Actions runner disk space

## [0.1.4] - 2026-01-03

### Added
- `#[rustapi_rs::schema]` attribute macro for opt-in OpenAPI schema auto-registration

### Changed
- Internal workspace dependency pins aligned to the workspace version for consistent publishing
- Proof-of-concept example includes a minimal `GET /` landing page

## [0.1.3] - 2026-01-01

### Added
- **New `rustapi-toon` crate**: TOON (Token-Oriented Object Notation) format support
  - LLM-optimized data serialization format
  - Content negotiation via `Accept` header (`application/toon`, `application/json`)
  - `Toon<T>` extractor and responder
  - `ToonNegotiate<T>` for automatic format selection
  - `LlmResponse<T>` for AI-friendly structured responses
  - OpenAPI integration with TOON schema support
- `toon` feature flag in `rustapi-rs` for opt-in TOON support
- `toon-api` example demonstrating TOON format usage
- Improved cookie extraction test for duplicate cookie names

### Changed
- Updated `rustapi-rs` to re-export toon module when feature enabled

## [0.1.2] - 2024-12-31

### Added
- `skip_paths` method for JwtLayer to exclude paths from JWT validation
- `docs_with_auth` method for Basic Auth protected Swagger UI
- `docs_with_auth_and_info` method for customized protected docs

### Changed
- auth-api example now demonstrates protected docs with Basic Auth
- JWT middleware can now skip validation for public endpoints

## [0.1.1] - 2024-12-31

### Added

#### Phase 4: Ergonomics & v1.0 Preparation
- Body size limit middleware with configurable limits
- `.body_limit(size)` builder method on RustApi (default: 1MB)
- 413 Payload Too Large response for oversized requests
- Production error masking (`RUSTAPI_ENV=production`)
- Development error details (`RUSTAPI_ENV=development`)
- Unique error IDs (`err_{uuid}`) for log correlation
- Enhanced tracing layer with request_id, status, and duration
- Custom span field support via `.with_field(key, value)`
- Prometheus metrics middleware (feature-gated)
- `http_requests_total` counter with method, path, status labels
- `http_request_duration_seconds` histogram
- `rustapi_info` gauge with version information
- `/metrics` endpoint handler
- TestClient for integration testing without network binding
- TestRequest builder with method, header, and body support
- TestResponse with assertion helpers
- `RUSTAPI_DEBUG=1` macro expansion output support
- Improved route path validation at compile time
- Enhanced route conflict detection messages

### Changed
- Error responses now include `error_id` field
- TracingLayer enhanced with additional span fields

## [0.1.0] - 2024-12-01

### Added

#### Phase 1: MVP Core
- Core HTTP server built on tokio and hyper 1.0
- Radix-tree based routing with matchit
- Request extractors: `Json<T>`, `Query<T>`, `Path<T>`
- Response types with automatic serialization
- Async handler support
- Basic error handling with `ApiError`
- `#[rustapi_rs::get]`, `#[rustapi_rs::post]` route macros
- `#[rustapi_rs::main]` async main macro

#### Phase 2: Validation & OpenAPI
- Automatic OpenAPI spec generation
- Swagger UI at `/docs` endpoint
- Request validation with validator crate
- `#[validate]` attribute support
- 422 Unprocessable Entity for validation errors
- `#[rustapi_rs::tag]` and `#[rustapi_rs::summary]` macros
- Schema derivation for request/response types

#### Phase 3: Batteries Included
- JWT authentication middleware (`jwt` feature)
- `AuthUser<T>` extractor for authenticated routes
- CORS middleware with builder pattern (`cors` feature)
- IP-based rate limiting (`rate-limit` feature)
- Configuration management with `.env` support (`config` feature)
- Cookie parsing extractor (`cookies` feature)
- SQLx error conversion (`sqlx` feature)
- Request ID middleware
- Middleware layer trait for custom middleware
- `extras` meta-feature for common optional features
- `full` feature for all optional features

[Unreleased]: https://github.com/Tuntii/RustAPI/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/Tuntii/RustAPI/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/Tuntii/RustAPI/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Tuntii/RustAPI/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Tuntii/RustAPI/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Tuntii/RustAPI/releases/tag/v0.1.0
