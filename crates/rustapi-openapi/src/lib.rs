//! OpenAPI documentation for RustAPI
//!
//! This crate provides OpenAPI specification generation and Swagger UI serving
//! for RustAPI applications. It supports both native schema generation and
//! optional `utoipa` integration for backward compatibility.
//!
//! # Features
//!
//! - **OpenAPI 3.0.3** and **OpenAPI 3.1.0** specification support
//! - **Native schema generation** without external dependencies
//! - Swagger UI serving at `/docs`
//! - JSON spec at `/openapi.json`
//! - Schema derivation via native `ToOpenApiSchema` trait
//! - **API versioning** with multiple strategies (path, header, query, accept)
//! - **JSON Schema 2020-12** support for OpenAPI 3.1
//! - **Webhook definitions** support
//! - Optional `utoipa` support for backward compatibility
//!
//! # Native Schema Generation (Recommended)
//!
//! ```rust,ignore
//! use rustapi_openapi::native::{ToOpenApiSchema, NativeSchema, ObjectSchemaBuilder};
//!
//! // Use the builder API for schemas
//! let user_schema = ObjectSchemaBuilder::new()
//!     .title("User")
//!     .required_integer("id")
//!     .required_string("name")
//!     .optional_string("email")
//!     .build();
//!
//! // Or implement ToOpenApiSchema for your types
//! impl ToOpenApiSchema for User {
//!     fn schema() -> (std::borrow::Cow<'static, str>, serde_json::Value) {
//!         ("User".into(), user_schema)
//!     }
//! }
//! ```
//!
//! # OpenAPI 3.1 Usage
//!
//! ```rust,ignore
//! use rustapi_openapi::v31::{OpenApi31Spec, Webhook, JsonSchema2020};
//!
//! let spec = OpenApi31Spec::new("My API", "1.0.0")
//!     .description("API with OpenAPI 3.1 support")
//!     .webhook("orderPlaced", Webhook::with_summary("Order notification"))
//!     .schema("User", JsonSchema2020::object()
//!         .with_property("id", JsonSchema2020::integer())
//!         .with_property("name", JsonSchema2020::string())
//!         .with_required("id"))
//!     .build();
//! ```
//!
//! # API Versioning Usage
//!
//! ```rust,ignore
//! use rustapi_openapi::versioning::{VersionRouter, ApiVersion, VersionStrategy};
//!
//! let router = VersionRouter::new()
//!     .strategy(VersionStrategy::path())
//!     .default_version(ApiVersion::v1())
//!     .version(ApiVersion::v1(), VersionedRouteConfig::version(ApiVersion::v1()))
//!     .version(ApiVersion::v2(), VersionedRouteConfig::version(ApiVersion::v2()));
//! ```
//!
//! # Legacy Usage with utoipa (requires "utoipa" feature)
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//!
//! #[derive(Serialize, Schema)]
//! struct User {
//!     id: i64,
//!     name: String,
//! }
//!
//! RustApi::new()
//!     .route("/users", get(list_users))
//!     .docs("/docs")
//!     .run("127.0.0.1:8080")
//!     .await
//! ```

mod config;
#[cfg(feature = "redoc")]
mod redoc;
mod schemas;
mod spec;
#[cfg(feature = "swagger-ui")]
mod swagger;

// Native OpenAPI schema generation (no external dependencies)
pub mod native;

// OpenAPI 3.1 support
pub mod v31;

// API versioning support
pub mod versioning;

pub use config::OpenApiConfig;
pub use schemas::{
    ErrorBodySchema, ErrorSchema, FieldErrorSchema, ValidationErrorBodySchema,
    ValidationErrorSchema,
};
pub use spec::{
    ApiInfo, MediaType, OpenApiSpec, Operation, OperationModifier, Parameter, PathItem,
    RequestBody, ResponseModifier, ResponseSpec, SchemaRef,
};

// Re-export native schema types at crate root for convenience
pub use native::{
    IntoOpenApiParams, NativeSchema, NativeSchemaBuilder, ObjectSchemaBuilder, ParamInfo,
    ParamLocation, PropertyInfo, SchemaFormat, SchemaType, ToOpenApiSchema,
};

// Re-export utoipa's ToSchema derive macro as Schema (for backward compatibility)
#[cfg(feature = "utoipa")]
pub use utoipa::ToSchema as Schema;
// Re-export utoipa's IntoParams derive macro (for backward compatibility)
#[cfg(feature = "utoipa")]
pub use utoipa::IntoParams;

// Re-export utoipa types for advanced usage (only when utoipa feature is enabled)
#[cfg(feature = "utoipa")]
pub mod utoipa_types {
    pub use utoipa::{openapi, IntoParams, Modify, OpenApi, ToSchema};
}

use bytes::Bytes;
use http::{header, Response, StatusCode};
use http_body_util::Full;

/// Generate OpenAPI JSON response
pub fn openapi_json(spec: &OpenApiSpec) -> Response<Full<Bytes>> {
    match serde_json::to_string_pretty(&spec.to_json()) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from("Failed to serialize OpenAPI spec")))
            .unwrap(),
    }
}

/// Generate Swagger UI HTML response
#[cfg(feature = "swagger-ui")]
pub fn swagger_ui_html(openapi_url: &str) -> Response<Full<Bytes>> {
    let html = swagger::generate_swagger_html(openapi_url);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

/// Generate ReDoc HTML response
///
/// ReDoc provides a three-panel API documentation interface.
///
/// # Example
/// ```rust,ignore
/// use rustapi_openapi::redoc_html;
/// let response = redoc_html("/openapi.json");
/// ```
#[cfg(feature = "redoc")]
pub fn redoc_html(openapi_url: &str) -> Response<Full<Bytes>> {
    let html = redoc::generate_redoc_html(openapi_url, None);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

/// Generate ReDoc HTML response with custom configuration
#[cfg(feature = "redoc")]
pub fn redoc_html_with_config(
    openapi_url: &str,
    title: Option<&str>,
    config: &redoc::RedocConfig,
) -> Response<Full<Bytes>> {
    let html = redoc::generate_redoc_html_with_config(openapi_url, title, config);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

#[cfg(feature = "redoc")]
pub use redoc::{RedocConfig, RedocTheme};

/// Generate OpenAPI 3.1 JSON response
pub fn openapi_31_json(spec: &v31::OpenApi31Spec) -> Response<Full<Bytes>> {
    match serde_json::to_string_pretty(&spec) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from(
                "Failed to serialize OpenAPI 3.1 spec",
            )))
            .unwrap(),
    }
}
