//! # RustAPI OpenAPI
//!
//! OpenAPI documentation generation for RustAPI framework.
//! Provides automatic OpenAPI spec generation and Swagger UI.
//!
//! ## Features
//!
//! - Automatic OpenAPI 3.1 spec generation
//! - Built-in Swagger UI at `/docs`
//! - Schema generation from Rust types
//! - Validation rules → OpenAPI constraints
//!
//! ## Example
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_openapi::OpenApiDoc;
//!
//! #[derive(Serialize, ToSchema)]
//! struct User {
//!     id: i64,
//!     name: String,
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     RustApi::new()
//!         .route("/users", get(list_users))
//!         .openapi(OpenApiDoc::new("My API", "1.0.0"))
//!         .run("127.0.0.1:8080")
//!         .await;
//! }
//! ```

mod doc;
mod schema;
mod swagger;

pub use doc::OpenApiDoc;
pub use schema::ToSchema;
pub use swagger::swagger_ui_html;

// Re-export utoipa derive for users
pub use utoipa::ToSchema as UtoipaToSchema;
pub use utoipa::OpenApi as UtoipaOpenApi;

/// Prelude module for OpenAPI
pub mod prelude {
    pub use crate::doc::OpenApiDoc;
    pub use crate::schema::ToSchema;
    pub use utoipa::ToSchema as UtoipaToSchema;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_doc_creation() {
        let doc = OpenApiDoc::new("Test API", "1.0.0");
        assert_eq!(doc.title(), "Test API");
        assert_eq!(doc.version(), "1.0.0");
    }
}
