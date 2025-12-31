//! # RustAPI
//!
//! A FastAPI-like web framework for Rust.
//!
//! RustAPI combines Rust's performance and safety with FastAPI's "just write business logic"
//! approach. It provides automatic OpenAPI documentation, declarative validation, and
//! a developer-friendly experience.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//!
//! #[derive(Serialize, Schema)]
//! struct Hello {
//!     message: String,
//! }
//!
//! async fn hello() -> Json<Hello> {
//!     Json(Hello {
//!         message: "Hello, World!".to_string(),
//!     })
//! }
//!
//! #[tokio::main]
//! async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     RustApi::new()
//!         .route("/", get(hello))
//!         .run("127.0.0.1:8080")
//!         .await
//! }
//! ```
//!
//! ## Features
//!
//! - **DX-First**: Minimal boilerplate, intuitive API
//! - **Type-Safe**: Compile-time route and schema validation
//! - **Auto Documentation**: OpenAPI + Swagger UI out of the box
//! - **Declarative Validation**: Pydantic-style validation on structs
//! - **Batteries Included**: JWT, CORS, rate limiting (optional features)
//!
//! ## Optional Features
//!
//! Enable these features in your `Cargo.toml`:
//!
//! - `jwt` - JWT authentication middleware and `AuthUser<T>` extractor
//! - `cors` - CORS middleware with builder pattern configuration
//! - `rate-limit` - IP-based rate limiting middleware
//! - `config` - Configuration management with `.env` file support
//! - `cookies` - Cookie parsing extractor
//! - `sqlx` - SQLx database error conversion to ApiError
//! - `extras` - Meta feature enabling jwt, cors, and rate-limit
//! - `full` - All optional features enabled
//!
//! ```toml
//! [dependencies]
//! rustapi-rs = { version = "0.1", features = ["jwt", "cors"] }
//! ```

// Re-export core functionality
pub use rustapi_core::*;

// Re-export macros
pub use rustapi_macros::*;

// Re-export extras (feature-gated)
#[cfg(feature = "jwt")]
pub use rustapi_extras::jwt;
#[cfg(feature = "jwt")]
pub use rustapi_extras::{create_token, AuthUser, JwtError, JwtLayer, JwtValidation, ValidatedClaims};

#[cfg(feature = "cors")]
pub use rustapi_extras::cors;
#[cfg(feature = "cors")]
pub use rustapi_extras::{AllowedOrigins, CorsLayer};

#[cfg(feature = "rate-limit")]
pub use rustapi_extras::rate_limit;
#[cfg(feature = "rate-limit")]
pub use rustapi_extras::RateLimitLayer;

#[cfg(feature = "config")]
pub use rustapi_extras::config;
#[cfg(feature = "config")]
pub use rustapi_extras::{env_or, env_parse, load_dotenv, load_dotenv_from, require_env, Config, ConfigError, Environment};

#[cfg(feature = "sqlx")]
pub use rustapi_extras::sqlx;
#[cfg(feature = "sqlx")]
pub use rustapi_extras::{convert_sqlx_error, SqlxErrorExt};

/// Prelude module - import everything you need with `use rustapi_rs::prelude::*`
pub mod prelude {
    // Core types
    pub use rustapi_core::{
        // App builder
        RustApi,
        // Router
        Router,
        get, post, put, patch, delete,
        // Route type for macro-based routing
        Route,
        get_route, post_route, put_route, patch_route, delete_route,
        // Extractors
        Json, Query, Path, State, Body,
        ValidatedJson,
        Headers, HeaderValue, ClientIp, Extension,
        // Response types
        IntoResponse, Response,
        Created, NoContent, Html, Redirect, WithStatus,
        // Streaming responses
        Sse, SseEvent, StreamBody,
        // Error handling
        ApiError, Result,
        // Request context
        Request,
        // Middleware
        RequestId, RequestIdLayer, TracingLayer,
    };

    // Cookies extractor (feature-gated in core)
    #[cfg(feature = "cookies")]
    pub use rustapi_core::Cookies;

    // Re-export the route! macro
    pub use rustapi_core::route;

    // Re-export validation - use validator derive macro directly
    pub use validator::Validate;
    
    // Re-export OpenAPI schema derive
    pub use rustapi_openapi::{Schema, IntoParams};

    // Re-export commonly used external types
    pub use serde::{Deserialize, Serialize};
    pub use tracing::{debug, error, info, trace, warn};

    // JWT types (feature-gated)
    #[cfg(feature = "jwt")]
    pub use rustapi_extras::{AuthUser, JwtLayer, JwtValidation, JwtError, ValidatedClaims, create_token};

    // CORS types (feature-gated)
    #[cfg(feature = "cors")]
    pub use rustapi_extras::{CorsLayer, AllowedOrigins};

    // Rate limiting types (feature-gated)
    #[cfg(feature = "rate-limit")]
    pub use rustapi_extras::RateLimitLayer;

    // Configuration types (feature-gated)
    #[cfg(feature = "config")]
    pub use rustapi_extras::{Config, Environment, ConfigError, load_dotenv, load_dotenv_from, env_or, env_parse, require_env};

    // SQLx types (feature-gated)
    #[cfg(feature = "sqlx")]
    pub use rustapi_extras::{convert_sqlx_error, SqlxErrorExt};
}

#[cfg(test)]
mod tests {
    use super::prelude::*;

    #[test]
    fn prelude_imports_work() {
        // This test ensures prelude exports compile correctly
        let _: fn() -> Result<()> = || Ok(());
    }
}
