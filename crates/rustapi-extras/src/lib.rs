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
//! - `config` - Configuration management with `.env` file support
//! - `cookies` - Cookie parsing extractor
//! - `sqlx` - SQLx database error conversion to ApiError
//! - `insight` - Traffic insight middleware for analytics and debugging
//! - `extras` - Meta feature enabling jwt, cors, and rate-limit
//! - `full` - All features enabled
//!
//! ## Example
//!
//! ```toml
//! [dependencies]
//! rustapi-extras = { version = "0.1", features = ["jwt", "cors", "insight"] }
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

// Traffic insight module
#[cfg(feature = "insight")]
pub mod insight;

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
pub use sqlx::{convert_sqlx_error, SqlxErrorExt};

#[cfg(feature = "insight")]
pub use insight::{
    InMemoryInsightStore, InsightConfig, InsightData, InsightLayer, InsightStats, InsightStore,
};
