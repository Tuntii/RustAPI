# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- `#[rustapi::get]`, `#[rustapi::post]` route macros
- `#[rustapi::main]` async main macro

#### Phase 2: Validation & OpenAPI
- Automatic OpenAPI spec generation
- Swagger UI at `/docs` endpoint
- Request validation with validator crate
- `#[validate]` attribute support
- 422 Unprocessable Entity for validation errors
- `#[rustapi::tag]` and `#[rustapi::summary]` macros
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
