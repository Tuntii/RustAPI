//! # rustapi-extras
//!
//! Optional security and utility features for the RustAPI framework.
//!
//! This crate provides production-ready middleware and utilities that are
//! opt-in via Cargo feature flags to minimize binary size when not needed.
//!
//! ## Features
//!
//! - `jwt` - JWT authentication middleware and `AuthUser<T>` extractor
//! - `cors` - CORS middleware with builder pattern configuration
//! - `rate-limit` - IP-based rate limiting middleware
//! - `csrf` - CSRF protection using Double-Submit Cookie pattern
//! - `config` - Configuration management with `.env` file support
//! - `cookies` - Cookie parsing extractor
//! - `sqlx` - SQLx database error conversion to ApiError
//! - `insight` - Traffic insight middleware for analytics and debugging
//! - `extras` - Meta feature enabling jwt, cors, and rate-limit
//! - `full` - All features enabled
//!
//! ## CSRF Protection Example
//!
//! ```rust,ignore
//! use rustapi_core::RustApi;
//! use rustapi_extras::csrf::{CsrfConfig, CsrfLayer, CsrfToken};
//!
//! let config = CsrfConfig::new()
//!     .cookie_name("csrf_token")
//!     .header_name("X-CSRF-Token");
//!
//! let app = RustApi::new()
//!     .layer(CsrfLayer::new(config));
//! ```
//!
//! ## Example
//!
//! ```toml
//! [dependencies]
//! rustapi-extras = { version = "0.1", features = ["jwt", "cors", "csrf"] }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

// JWT authentication module
#[cfg(feature = "jwt")]
pub mod jwt;

// CORS middleware module
#[cfg(feature = "cors")]
pub mod cors;

// Rate limiting module
#[cfg(feature = "rate-limit")]
pub mod rate_limit;

// Configuration management module
#[cfg(feature = "config")]
pub mod config;

// SQLx database integration module
#[cfg(feature = "sqlx")]
pub mod sqlx;

// Diesel database integration module
#[cfg(feature = "diesel")]
pub mod diesel;

// Traffic insight module
#[cfg(feature = "insight")]
pub mod insight;

// Request timeout middleware
#[cfg(feature = "timeout")]
pub mod timeout;

// Request guards (authorization)
#[cfg(feature = "guard")]
pub mod guard;

// Request/Response logging middleware
#[cfg(feature = "logging")]
pub mod logging;

// Circuit breaker middleware
#[cfg(feature = "circuit-breaker")]
pub mod circuit_breaker;

// Retry middleware
#[cfg(feature = "retry")]
pub mod retry;

// Request deduplication
#[cfg(feature = "dedup")]
pub mod dedup;

// Input sanitization
#[cfg(feature = "sanitization")]
pub mod sanitization;

// Security headers middleware
#[cfg(feature = "security-headers")]
pub mod security_headers;

// API Key authentication
#[cfg(feature = "api-key")]
pub mod api_key;

// Response caching
#[cfg(feature = "cache")]
pub mod cache;

// OpenTelemetry integration
#[cfg(feature = "otel")]
pub mod otel;

// Structured logging
#[cfg(feature = "structured-logging")]
pub mod structured_logging;

// Re-exports for convenience
#[cfg(feature = "jwt")]
pub use jwt::{create_token, AuthUser, JwtError, JwtLayer, JwtValidation, ValidatedClaims};

#[cfg(feature = "cors")]
pub use cors::{AllowedOrigins, CorsLayer};

#[cfg(feature = "rate-limit")]
pub use rate_limit::RateLimitLayer;

#[cfg(feature = "config")]
pub use config::{
    env_or, env_parse, load_dotenv, load_dotenv_from, require_env, try_require_env, Config,
    ConfigError, Environment,
};

#[cfg(feature = "sqlx")]
pub use sqlx::{convert_sqlx_error, PoolError, SqlxErrorExt, SqlxPoolBuilder, SqlxPoolConfig};

#[cfg(feature = "diesel")]
pub use diesel::{DieselPoolBuilder, DieselPoolConfig, DieselPoolError};

#[cfg(feature = "insight")]
pub use insight::{
    InMemoryInsightStore, InsightConfig, InsightData, InsightLayer, InsightStats, InsightStore,
};

// Phase 11 re-exports
#[cfg(feature = "timeout")]
pub use timeout::TimeoutLayer;

#[cfg(feature = "guard")]
pub use guard::{PermissionGuard, RoleGuard};

#[cfg(feature = "logging")]
pub use logging::{LogFormat, LoggingConfig, LoggingLayer};

#[cfg(feature = "circuit-breaker")]
pub use circuit_breaker::{CircuitBreakerLayer, CircuitBreakerStats, CircuitState};

#[cfg(feature = "retry")]
pub use retry::{RetryLayer, RetryStrategy};

#[cfg(feature = "security-headers")]
pub use security_headers::{HstsConfig, ReferrerPolicy, SecurityHeadersLayer, XFrameOptions};

#[cfg(feature = "api-key")]
pub use api_key::ApiKeyLayer;

#[cfg(feature = "cache")]
pub use cache::{CacheConfig, CacheLayer};

#[cfg(feature = "dedup")]
pub use dedup::{DedupConfig, DedupLayer};

#[cfg(feature = "sanitization")]
pub use sanitization::{sanitize_html, sanitize_json, strip_tags};

// Phase 5: Observability re-exports
#[cfg(feature = "otel")]
pub use otel::{
    extract_trace_context, inject_trace_context, propagate_trace_context, OtelConfig,
    OtelConfigBuilder, OtelExporter, OtelLayer, TraceContext, TraceSampler,
};

#[cfg(feature = "structured-logging")]
pub use structured_logging::{
    DatadogFormatter, JsonFormatter, LogFormatter, LogOutputFormat, LogfmtFormatter,
    SplunkFormatter, StructuredLoggingConfig, StructuredLoggingConfigBuilder,
    StructuredLoggingLayer,
};

// Phase 6: Security features
#[cfg(feature = "csrf")]
pub mod csrf;

#[cfg(feature = "csrf")]
pub use csrf::{CsrfConfig, CsrfLayer, CsrfToken};

#[cfg(feature = "oauth2-client")]
pub mod oauth2;

#[cfg(feature = "oauth2-client")]
pub use oauth2::{
    AuthorizationRequest, CsrfState, OAuth2Client, OAuth2Config, PkceVerifier, Provider,
    TokenError, TokenResponse,
};

#[cfg(feature = "audit")]
pub mod audit;

#[cfg(feature = "audit")]
pub use audit::{
    AuditAction, AuditEvent, AuditQuery, AuditQueryBuilder, AuditSeverity, AuditStore,
    ComplianceInfo, FileAuditStore, InMemoryAuditStore,
};

// Replay middleware (time-travel debugging)
#[cfg(feature = "replay")]
pub mod replay;

#[cfg(feature = "replay")]
pub use replay::{
    FsReplayStore, FsReplayStoreConfig, InMemoryReplayStore, ReplayAdminAuth, ReplayClient,
    ReplayLayer, RetentionJob,
};
