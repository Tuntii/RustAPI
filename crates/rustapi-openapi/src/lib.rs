//! OpenAPI documentation for RustAPI
//!
//! This crate provides OpenAPI specification generation and Swagger UI serving
//! for RustAPI applications. It wraps `utoipa` internally while providing a
//! clean public API.
//!
//! # Features
//!
//! - Automatic OpenAPI spec generation
//! - Swagger UI serving at `/docs`
//! - JSON spec at `/openapi.json`
//! - Schema derivation via `#[derive(Schema)]`
//!
//! # Usage
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

mod spec;
mod swagger;
mod config;
mod schemas;

pub use config::OpenApiConfig;
pub use spec::{OpenApiSpec, ApiInfo, Operation, PathItem, Parameter, ResponseSpec, OperationModifier, RequestBody, MediaType, SchemaRef, ResponseModifier};
pub use schemas::{ErrorSchema, ValidationErrorSchema, FieldErrorSchema};

// Re-export utoipa's ToSchema derive macro as Schema
pub use utoipa::ToSchema as Schema;

// Re-export utoipa types for advanced usage
pub mod utoipa_types {
    pub use utoipa::{ToSchema, OpenApi, Modify, openapi};
}

use bytes::Bytes;
use http::{Response, StatusCode, header};
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
pub fn swagger_ui_html(openapi_url: &str) -> Response<Full<Bytes>> {
    let html = swagger::generate_swagger_html(openapi_url);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

