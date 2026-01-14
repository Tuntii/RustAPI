//! CSRF Protection Module
//!
//! This module implements Double-Submit Cookie CSRF protection.
//!
//! # Example
//!
//! ```rust,no_run
//! use rustapi_core::RustApi;
//! use rustapi_extras::csrf::{CsrfConfig, CsrfLayer};
//!
//! let config = CsrfConfig::new()
//!     .cookie_name("my-csrf-cookie")
//!     .header_name("X-CSRF-TOKEN");
//!
//! let app = RustApi::new()
//!     .layer(CsrfLayer::new(config));
//! ```

mod config;
mod layer;
mod token;

pub use config::CsrfConfig;
pub use layer::CsrfLayer;
pub use token::CsrfToken;
