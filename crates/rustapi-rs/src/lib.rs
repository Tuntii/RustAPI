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
//! - `legacy-validator` - Compatibility mode for `validator::Validate`
//! - `extras` - Meta feature enabling jwt, cors, and rate-limit
//! - `full` - All optional features enabled
//!
//! ```toml
//! [dependencies]
//! rustapi-rs = { version = "0.1.300", features = ["jwt", "cors"] }
//! ```

// Re-export core functionality
pub use rustapi_core::*;

// Re-export macros
pub use rustapi_macros::*;

// Re-export extras (feature-gated)
#[cfg(feature = "jwt")]
pub use rustapi_extras::jwt;
#[cfg(feature = "jwt")]
pub use rustapi_extras::{
    create_token, AuthUser, JwtError, JwtLayer, JwtValidation, ValidatedClaims,
};

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
pub use rustapi_extras::{
    env_or, env_parse, load_dotenv, load_dotenv_from, require_env, Config, ConfigError, Environment,
};

#[cfg(feature = "sqlx")]
pub use rustapi_extras::{convert_sqlx_error, SqlxErrorExt};

// Re-export Phase 11 & Observability Features
#[cfg(feature = "timeout")]
pub use rustapi_extras::timeout;

#[cfg(feature = "guard")]
pub use rustapi_extras::guard;

#[cfg(feature = "logging")]
pub use rustapi_extras::logging;

#[cfg(feature = "circuit-breaker")]
pub use rustapi_extras::circuit_breaker;

#[cfg(feature = "retry")]
pub use rustapi_extras::retry;

#[cfg(feature = "security-headers")]
pub use rustapi_extras::security_headers;

#[cfg(feature = "api-key")]
pub use rustapi_extras::api_key;

#[cfg(feature = "cache")]
pub use rustapi_extras::cache;

#[cfg(feature = "dedup")]
pub use rustapi_extras::dedup;

#[cfg(feature = "sanitization")]
pub use rustapi_extras::sanitization;

#[cfg(feature = "otel")]
pub use rustapi_extras::otel;

#[cfg(feature = "structured-logging")]
pub use rustapi_extras::structured_logging;

// Replay (time-travel debugging)
#[cfg(feature = "replay")]
pub use rustapi_extras::replay;

// Re-export TOON (feature-gated)
#[cfg(feature = "toon")]
pub mod toon {
    //! TOON (Token-Oriented Object Notation) support
    //!
    //! TOON is a compact format for LLM communication that reduces token usage by 20-40%.
    //!
    //! # Example
    //!
    //! ```rust,ignore
    //! use rustapi_rs::toon::{Toon, Negotiate, AcceptHeader};
    //!
    //! // As extractor
    //! async fn handler(Toon(data): Toon<MyType>) -> impl IntoResponse { ... }
    //!
    //! // As response
    //! async fn handler() -> Toon<MyType> { Toon(my_data) }
    //!
    //! // Content negotiation (returns JSON or TOON based on Accept header)
    //! async fn handler(accept: AcceptHeader) -> Negotiate<MyType> {
    //!     Negotiate::new(my_data, accept.preferred)
    //! }
    //! ```
    pub use rustapi_toon::*;
}

// Re-export WebSocket support (feature-gated)
#[cfg(feature = "ws")]
pub mod ws {
    //! WebSocket support for real-time bidirectional communication
    //!
    //! This module provides WebSocket functionality through the `WebSocket` extractor,
    //! enabling real-time communication patterns like chat, live updates, and streaming.
    //!
    //! # Example
    //!
    //! ```rust,ignore
    //! use rustapi_rs::ws::{WebSocket, Message};
    //!
    //! async fn websocket_handler(ws: WebSocket) -> impl IntoResponse {
    //!     ws.on_upgrade(|mut socket| async move {
    //!         while let Some(Ok(msg)) = socket.recv().await {
    //!             if let Message::Text(text) = msg {
    //!                 socket.send(Message::Text(format!("Echo: {}", text))).await.ok();
    //!             }
    //!         }
    //!     })
    //! }
    //! ```
    pub use rustapi_ws::*;
}

// Re-export View/Template support (feature-gated)
#[cfg(feature = "view")]
pub mod view {
    //! Template engine support for server-side rendering
    //!
    //! This module provides Tera-based templating with the `View<T>` response type,
    //! enabling server-side HTML rendering with template inheritance and context.
    //!
    //! # Example
    //!
    //! ```rust,ignore
    //! use rustapi_rs::view::{Templates, View, ContextBuilder};
    //!
    //! #[derive(Clone)]
    //! struct AppState {
    //!     templates: Templates,
    //! }
    //!
    //! async fn index(State(state): State<AppState>) -> View<()> {
    //!     View::new(&state.templates, "index.html")
    //!         .with("title", "Home")
    //!         .with("message", "Welcome!")
    //! }
    //! ```
    pub use rustapi_view::*;
}

/// Prelude module - import everything you need with `use rustapi_rs::prelude::*`
pub mod prelude {
    // Core types
    pub use rustapi_core::validation::Validatable;
    pub use rustapi_core::{
        delete,
        delete_route,
        get,
        get_route,
        patch,
        patch_route,
        post,
        post_route,
        put,
        put_route,
        serve_dir,
        sse_response,
        // Error handling
        ApiError,
        AsyncValidatedJson,
        Body,
        ClientIp,
        Created,
        Extension,
        HeaderValue,
        Headers,
        Html,
        // Response types
        IntoResponse,
        // Extractors
        Json,
        KeepAlive,
        // Multipart
        Multipart,
        MultipartConfig,
        MultipartField,
        NoContent,
        Path,
        Query,
        Redirect,
        // Request context
        Request,
        // Middleware
        RequestId,
        RequestIdLayer,
        Response,
        Result,
        // Route type for macro-based routing
        Route,
        // Router
        Router,
        // App builder
        RustApi,
        RustApiConfig,
        // Streaming responses
        Sse,
        SseEvent,
        State,
        // Static files
        StaticFile,
        StaticFileConfig,
        StatusCode,
        StreamBody,
        TracingLayer,
        Typed,
        TypedPath,
        UploadedFile,
        ValidatedJson,
        WithStatus,
    };

    // Compression middleware (feature-gated in core)
    #[cfg(feature = "compression")]
    pub use rustapi_core::middleware::{CompressionAlgorithm, CompressionConfig};
    #[cfg(feature = "compression")]
    pub use rustapi_core::CompressionLayer;

    // Cookies extractor (feature-gated in core)
    #[cfg(feature = "cookies")]
    pub use rustapi_core::Cookies;

    // Re-export the route! macro
    pub use rustapi_core::route;

    // Re-export TypedPath derive macro
    pub use rustapi_macros::ApiError;
    pub use rustapi_macros::TypedPath;

    // Re-export validation - use validator derive macro directly
    pub use rustapi_validate::v2::AsyncValidate;
    pub use rustapi_validate::v2::Validate as V2Validate;
    #[cfg(feature = "legacy-validator")]
    pub use validator::Validate;

    // Re-export OpenAPI schema derive
    pub use rustapi_openapi::Schema;

    // Re-export crates needed by Schema derive macro
    // These are required for the macro-generated code to compile
    pub use rustapi_openapi;
    pub use serde_json;

    // Re-export commonly used external types
    pub use serde::{Deserialize, Serialize};
    pub use tracing::{debug, error, info, trace, warn};

    // JWT types (feature-gated)
    #[cfg(feature = "jwt")]
    pub use rustapi_extras::{
        create_token, AuthUser, JwtError, JwtLayer, JwtValidation, ValidatedClaims,
    };

    // CORS types (feature-gated)
    #[cfg(feature = "cors")]
    pub use rustapi_extras::{AllowedOrigins, CorsLayer};

    // Rate limiting types (feature-gated)
    #[cfg(feature = "rate-limit")]
    pub use rustapi_extras::RateLimitLayer;

    // Configuration types (feature-gated)
    #[cfg(feature = "config")]
    pub use rustapi_extras::{
        env_or, env_parse, load_dotenv, load_dotenv_from, require_env, Config, ConfigError,
        Environment,
    };

    // SQLx types (feature-gated)
    #[cfg(feature = "sqlx")]
    pub use rustapi_extras::{convert_sqlx_error, SqlxErrorExt};

    // TOON types (feature-gated)
    #[cfg(feature = "toon")]
    pub use rustapi_toon::{AcceptHeader, LlmResponse, Negotiate, OutputFormat, Toon};

    // WebSocket types (feature-gated)
    #[cfg(feature = "ws")]
    pub use rustapi_ws::{Broadcast, Message, WebSocket, WebSocketStream};

    // View/Template types (feature-gated)
    #[cfg(feature = "view")]
    pub use rustapi_view::{ContextBuilder, Templates, TemplatesConfig, View};
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
