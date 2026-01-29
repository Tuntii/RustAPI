//! Native OpenAPI schema generation without external dependencies
//!
//! This module provides native traits and implementations for generating
//! OpenAPI schemas without relying on external crates like `utoipa`.
//!
//! # Key Features
//!
//! - `ToOpenApiSchema` trait for types that can be converted to OpenAPI schemas
//! - Built-in implementations for primitive types and standard library types
//! - Schema composition with allOf, anyOf, oneOf
//! - Support for nested objects, arrays, enums, and optional fields
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_openapi::native::ToOpenApiSchema;
//!
//! // Implement for custom types
//! impl ToOpenApiSchema for User {
//!     fn schema() -> (std::borrow::Cow<'static, str>, serde_json::Value) {
//!         (
//!             "User".into(),
//!             serde_json::json!({
//!                 "type": "object",
//!                 "properties": {
//!                     "id": { "type": "integer", "format": "int64" },
//!                     "name": { "type": "string" }
//!                 },
//!                 "required": ["id", "name"]
//!             })
//!         )
//!     }
//! }
//! ```

mod schema;
mod traits;

#[cfg(test)]
mod tests;

pub use schema::{
    NativeSchema, NativeSchemaBuilder, ObjectSchemaBuilder, PropertyInfo, SchemaFormat, SchemaType,
};
pub use traits::{IntoOpenApiParams, ParamInfo, ParamLocation, ToOpenApiSchema};
