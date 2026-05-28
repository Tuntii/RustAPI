//! Template rendering support for RustAPI using Tera templates.
//!
//! This module provides server-side HTML rendering with type-safe template contexts,
//! layout inheritance, and development-friendly features like auto-reload.
//!
//! Available behind the `view` feature.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_extras::view::{View, Templates};
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
//! ```

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
    pub use super::{Context, ContextBuilder, Templates, TemplatesConfig, View, ViewError};
}
