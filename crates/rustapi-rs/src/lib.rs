//! # RustAPI
//!
//! Public facade crate for RustAPI.

extern crate self as rustapi_rs;

// Re-export all procedural macros from rustapi-macros.
pub use rustapi_macros::*;

/// Macro/runtime internals. Not part of the public compatibility contract.
#[doc(hidden)]
pub mod __private {
    pub use async_trait;
    pub use rustapi_core as core;
    pub use rustapi_core::__private::{linkme, AUTO_ROUTES, AUTO_SCHEMAS};
    pub use rustapi_openapi as openapi;
    pub use rustapi_validate as validate;
    pub use serde_json;
}

/// Stable core surface exposed by the facade.
pub mod core {
    pub use rustapi_core::collect_auto_routes;
    pub use rustapi_core::validation::Validatable;
    pub use rustapi_core::{
        delete, delete_route, get, get_route, patch, patch_route, post, post_route, put, put_route,
        route, serve_dir, sse_response, ApiError, AsyncValidatedJson, Body, BodyLimitLayer,
        BodyStream, BodyVariant, ClientIp, Created, Environment, Extension, FieldError,
        FromRequest, FromRequestParts, Handler, HandlerService, HeaderValue, Headers, Html,
        IntoResponse, Json, KeepAlive, MethodRouter, Multipart, MultipartConfig, MultipartField,
        NoContent, Path, Query, Redirect, Request, RequestId, RequestIdLayer, Response,
        ResponseBody, Result, Route, RouteHandler, RouteMatch, Router, RustApi, RustApiConfig, Sse,
        SseEvent, State, StaticFile, StaticFileConfig, StatusCode, StreamBody, TracingLayer, Typed,
        TypedPath, UploadedFile, ValidatedJson, WithStatus,
    };

    pub use rustapi_core::get_environment;

    #[cfg(any(feature = "core-cookies", feature = "cookies"))]
    pub use rustapi_core::Cookies;

    #[cfg(any(feature = "core-compression", feature = "compression"))]
    pub use rustapi_core::CompressionLayer;

    #[cfg(any(feature = "core-compression", feature = "compression"))]
    pub use rustapi_core::middleware::{CompressionAlgorithm, CompressionConfig};

    #[cfg(any(feature = "core-http3", feature = "protocol-http3", feature = "http3"))]
    pub use rustapi_core::{Http3Config, Http3Server};
}

// Backward-compatible root re-exports.
pub use core::*;

/// Optional protocol integrations grouped under a stable namespace.
pub mod protocol {
    #[cfg(any(feature = "protocol-toon", feature = "toon"))]
    pub mod toon {
        pub use rustapi_toon::*;
    }

    #[cfg(any(feature = "protocol-ws", feature = "ws"))]
    pub mod ws {
        pub use rustapi_ws::*;
    }

    #[cfg(any(feature = "protocol-view", feature = "view"))]
    pub mod view {
        pub use rustapi_view::*;
    }

    #[cfg(any(feature = "protocol-grpc", feature = "grpc"))]
    pub mod grpc {
        pub use rustapi_grpc::*;
    }

    #[cfg(any(feature = "core-http3", feature = "protocol-http3", feature = "http3"))]
    pub mod http3 {
        pub use rustapi_core::{Http3Config, Http3Server};
    }
}

/// Optional extras grouped under a stable namespace.
pub mod extras {
    #[cfg(any(feature = "extras-jwt", feature = "jwt"))]
    pub mod jwt {
        pub use rustapi_extras::jwt;
        pub use rustapi_extras::{
            create_token, AuthUser, JwtError, JwtLayer, JwtValidation, ValidatedClaims,
        };
    }

    #[cfg(any(feature = "extras-cors", feature = "cors"))]
    pub mod cors {
        pub use rustapi_extras::cors;
        pub use rustapi_extras::{AllowedOrigins, CorsLayer};
    }

    #[cfg(any(feature = "extras-rate-limit", feature = "rate-limit"))]
    pub mod rate_limit {
        pub use rustapi_extras::rate_limit;
        pub use rustapi_extras::RateLimitLayer;
    }

    #[cfg(any(feature = "extras-config", feature = "config"))]
    pub mod config {
        pub use rustapi_extras::config;
        pub use rustapi_extras::{
            env_or, env_parse, load_dotenv, load_dotenv_from, require_env, Config, ConfigError,
            Environment,
        };
    }

    #[cfg(any(feature = "extras-sqlx", feature = "sqlx"))]
    pub mod sqlx {
        pub use rustapi_extras::{convert_sqlx_error, SqlxErrorExt};
    }

    #[cfg(any(feature = "extras-insight", feature = "insight"))]
    pub mod insight {
        pub use rustapi_extras::insight;
    }

    #[cfg(any(feature = "extras-timeout", feature = "timeout"))]
    pub mod timeout {
        pub use rustapi_extras::timeout;
    }

    #[cfg(any(feature = "extras-guard", feature = "guard"))]
    pub mod guard {
        pub use rustapi_extras::guard;
    }

    #[cfg(any(feature = "extras-logging", feature = "logging"))]
    pub mod logging {
        pub use rustapi_extras::logging;
    }

    #[cfg(any(feature = "extras-circuit-breaker", feature = "circuit-breaker"))]
    pub mod circuit_breaker {
        pub use rustapi_extras::circuit_breaker;
    }

    #[cfg(any(feature = "extras-retry", feature = "retry"))]
    pub mod retry {
        pub use rustapi_extras::retry;
    }

    #[cfg(any(feature = "extras-security-headers", feature = "security-headers"))]
    pub mod security_headers {
        pub use rustapi_extras::security_headers;
    }

    #[cfg(any(feature = "extras-api-key", feature = "api-key"))]
    pub mod api_key {
        pub use rustapi_extras::api_key;
    }

    #[cfg(any(feature = "extras-cache", feature = "cache"))]
    pub mod cache {
        pub use rustapi_extras::cache;
    }

    #[cfg(any(feature = "extras-dedup", feature = "dedup"))]
    pub mod dedup {
        pub use rustapi_extras::dedup;
    }

    #[cfg(any(feature = "extras-sanitization", feature = "sanitization"))]
    pub mod sanitization {
        pub use rustapi_extras::sanitization;
    }

    #[cfg(any(feature = "extras-otel", feature = "otel"))]
    pub mod otel {
        pub use rustapi_extras::otel;
    }

    #[cfg(any(feature = "extras-structured-logging", feature = "structured-logging"))]
    pub mod structured_logging {
        pub use rustapi_extras::structured_logging;
    }

    #[cfg(any(feature = "extras-replay", feature = "replay"))]
    pub mod replay {
        pub use rustapi_extras::replay;
    }
}

// Legacy root module aliases.
#[cfg(any(feature = "protocol-toon", feature = "toon"))]
#[deprecated(note = "Use rustapi_rs::protocol::toon instead")]
pub mod toon {
    pub use crate::protocol::toon::*;
}

#[cfg(any(feature = "protocol-ws", feature = "ws"))]
#[deprecated(note = "Use rustapi_rs::protocol::ws instead")]
pub mod ws {
    pub use crate::protocol::ws::*;
}

#[cfg(any(feature = "protocol-view", feature = "view"))]
#[deprecated(note = "Use rustapi_rs::protocol::view instead")]
pub mod view {
    pub use crate::protocol::view::*;
}

#[cfg(any(feature = "protocol-grpc", feature = "grpc"))]
#[deprecated(note = "Use rustapi_rs::protocol::grpc instead")]
pub mod grpc {
    pub use crate::protocol::grpc::*;
}

// Legacy root extras re-exports for compatibility.
#[cfg(any(feature = "extras-jwt", feature = "jwt"))]
pub use rustapi_extras::jwt;
#[cfg(any(feature = "extras-jwt", feature = "jwt"))]
pub use rustapi_extras::{
    create_token, AuthUser, JwtError, JwtLayer, JwtValidation, ValidatedClaims,
};

#[cfg(any(feature = "extras-cors", feature = "cors"))]
pub use rustapi_extras::cors;
#[cfg(any(feature = "extras-cors", feature = "cors"))]
pub use rustapi_extras::{AllowedOrigins, CorsLayer};

#[cfg(any(feature = "extras-rate-limit", feature = "rate-limit"))]
pub use rustapi_extras::rate_limit;
#[cfg(any(feature = "extras-rate-limit", feature = "rate-limit"))]
pub use rustapi_extras::RateLimitLayer;

#[cfg(any(feature = "extras-config", feature = "config"))]
pub use rustapi_extras::config;
#[cfg(any(feature = "extras-config", feature = "config"))]
pub use rustapi_extras::{
    env_or, env_parse, load_dotenv, load_dotenv_from, require_env, Config, ConfigError,
    Environment as ExtrasEnvironment,
};

#[cfg(any(feature = "extras-sqlx", feature = "sqlx"))]
pub use rustapi_extras::{convert_sqlx_error, SqlxErrorExt};

#[cfg(any(feature = "extras-api-key", feature = "api-key"))]
pub use rustapi_extras::api_key;
#[cfg(any(feature = "extras-cache", feature = "cache"))]
pub use rustapi_extras::cache;
#[cfg(any(feature = "extras-circuit-breaker", feature = "circuit-breaker"))]
pub use rustapi_extras::circuit_breaker;
#[cfg(any(feature = "extras-dedup", feature = "dedup"))]
pub use rustapi_extras::dedup;
#[cfg(any(feature = "extras-guard", feature = "guard"))]
pub use rustapi_extras::guard;
#[cfg(any(feature = "extras-logging", feature = "logging"))]
pub use rustapi_extras::logging;
#[cfg(any(feature = "extras-otel", feature = "otel"))]
pub use rustapi_extras::otel;
#[cfg(any(feature = "extras-replay", feature = "replay"))]
pub use rustapi_extras::replay;
#[cfg(any(feature = "extras-retry", feature = "retry"))]
pub use rustapi_extras::retry;
#[cfg(any(feature = "extras-sanitization", feature = "sanitization"))]
pub use rustapi_extras::sanitization;
#[cfg(any(feature = "extras-security-headers", feature = "security-headers"))]
pub use rustapi_extras::security_headers;
#[cfg(any(feature = "extras-structured-logging", feature = "structured-logging"))]
pub use rustapi_extras::structured_logging;
#[cfg(any(feature = "extras-timeout", feature = "timeout"))]
pub use rustapi_extras::timeout;

/// Prelude module: `use rustapi_rs::prelude::*`.
pub mod prelude {
    pub use crate::core::Validatable;
    pub use crate::core::{
        delete, delete_route, get, get_route, patch, patch_route, post, post_route, put, put_route,
        route, serve_dir, sse_response, ApiError, AsyncValidatedJson, Body, BodyLimitLayer,
        ClientIp, Created, Extension, HeaderValue, Headers, Html, IntoResponse, Json, KeepAlive,
        Multipart, MultipartConfig, MultipartField, NoContent, Path, Query, Redirect, Request,
        RequestId, RequestIdLayer, Response, Result, Route, Router, RustApi, RustApiConfig, Sse,
        SseEvent, State, StaticFile, StaticFileConfig, StatusCode, StreamBody, TracingLayer, Typed,
        TypedPath, UploadedFile, ValidatedJson, WithStatus,
    };

    #[cfg(any(feature = "core-compression", feature = "compression"))]
    pub use crate::core::{CompressionAlgorithm, CompressionConfig, CompressionLayer};

    #[cfg(any(feature = "core-cookies", feature = "cookies"))]
    pub use crate::core::Cookies;

    pub use rustapi_macros::ApiError;
    pub use rustapi_macros::Schema;
    pub use rustapi_macros::TypedPath;

    pub use rustapi_validate::v2::AsyncValidate;
    pub use rustapi_validate::v2::Validate as V2Validate;

    #[cfg(any(feature = "core-legacy-validator", feature = "legacy-validator"))]
    pub use validator::Validate;

    pub use serde::{Deserialize, Serialize};
    pub use tracing::{debug, error, info, trace, warn};

    #[cfg(any(feature = "extras-jwt", feature = "jwt"))]
    pub use crate::{create_token, AuthUser, JwtError, JwtLayer, JwtValidation, ValidatedClaims};

    #[cfg(any(feature = "extras-cors", feature = "cors"))]
    pub use crate::{AllowedOrigins, CorsLayer};

    #[cfg(any(feature = "extras-rate-limit", feature = "rate-limit"))]
    pub use crate::RateLimitLayer;

    #[cfg(any(feature = "extras-config", feature = "config"))]
    pub use crate::{
        env_or, env_parse, load_dotenv, load_dotenv_from, require_env, Config, ConfigError,
        ExtrasEnvironment,
    };

    #[cfg(any(feature = "extras-sqlx", feature = "sqlx"))]
    pub use crate::{convert_sqlx_error, SqlxErrorExt};

    #[cfg(any(feature = "protocol-toon", feature = "toon"))]
    pub use crate::protocol::toon::{AcceptHeader, LlmResponse, Negotiate, OutputFormat, Toon};

    #[cfg(any(feature = "protocol-ws", feature = "ws"))]
    pub use crate::protocol::ws::{Broadcast, Message, WebSocket, WebSocketStream};

    #[cfg(any(feature = "protocol-view", feature = "view"))]
    pub use crate::protocol::view::{ContextBuilder, Templates, TemplatesConfig, View};

    #[cfg(any(feature = "protocol-grpc", feature = "grpc"))]
    pub use crate::protocol::grpc::{
        run_concurrently, run_rustapi_and_grpc, run_rustapi_and_grpc_with_shutdown,
    };
}

#[cfg(test)]
mod tests {
    use super::prelude::*;

    #[test]
    fn prelude_imports_work() {
        let _: fn() -> Result<()> = || Ok(());
    }
}
