//! # rustapi-view
//!
//! Template rendering support for the RustAPI framework using Tera templates.
//!
//! This crate provides server-side HTML rendering with type-safe template contexts,
//! layout inheritance, and development-friendly features like auto-reload.
//!
//! ## Features
//!
//! - **Tera Templates**: Full Tera template engine support with filters, macros, and inheritance
//! - **Type-Safe Context**: Build template context from Rust structs via serde
//! - **Auto-Reload**: Development mode can auto-reload templates on change
//! - **Response Types**: `View<T>` response type for rendering templates
//! - **Layout Support**: Template inheritance with blocks
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_view::{View, Templates};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct HomeContext {
//!     title: String,
//!     user: Option<String>,
//! }
//!
//! async fn home(templates: State<Templates>) -> View<HomeContext> {
//!     View::render(&templates, "home.html", HomeContext {
//!         title: "Welcome".to_string(),
//!         user: Some("Alice".to_string()),
//!     })
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let templates = Templates::new("templates/**/*.html")?;
//!
//!     RustApi::new()
//!         .state(templates)
//!         .route("/", get(home))
//!         .run("127.0.0.1:8080")
//!         .await
//! }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod context;
mod error;
mod templates;
mod view;

pub use context::ContextBuilder;
pub use error::ViewError;
pub use templates::{Templates, TemplatesConfig};
pub use view::View;

// Re-export tera types that users might need
pub use tera::Context;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{Context, ContextBuilder, Templates, TemplatesConfig, View, ViewError};
}
