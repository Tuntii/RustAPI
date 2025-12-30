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
//! ```rust,no_run
//! use rustapi_rs::prelude::*;
//!
//! #[derive(Serialize)]
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
//! async fn main() -> Result<()> {
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

// Re-export core functionality
pub use rustapi_core::*;

// Re-export macros
pub use rustapi_macros::*;

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
        // Response types
        IntoResponse, Response,
        Created, NoContent, Html, Redirect,
        // Error handling
        ApiError, Result,
        // Request context
        Request,
    };

    // Re-export the route! macro
    pub use rustapi_core::route;

    // Re-export validation - use validator derive macro directly
    pub use validator::Validate;

    // Re-export commonly used external types
    pub use serde::{Deserialize, Serialize};
    pub use tracing::{debug, error, info, trace, warn};
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
