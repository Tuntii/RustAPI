//! # RustAPI Validation
//!
//! Validation system for RustAPI framework. Provides declarative validation
//! on structs using the `#[derive(Validate)]` macro.
//!
//! ## Example
//!
//! ```rust,ignore
//! use rustapi_validate::prelude::*;
//! use validator::Validate;
//!
//! #[derive(Validate)]
//! struct CreateUser {
//!     #[validate(email)]
//!     email: String,
//!     
//!     #[validate(length(min = 3, max = 50))]
//!     username: String,
//!     
//!     #[validate(range(min = 18, max = 120))]
//!     age: u8,
//! }
//! ```
//!
//! ## V2 Validation Engine
//!
//! The v2 module provides a custom validation engine with async support:
//!
//! ```rust,ignore
//! use rustapi_validate::v2::prelude::*;
//!
//! #[derive(Validate)]
//! struct CreateUser {
//!     #[validate(email, message = "Invalid email format")]
//!     email: String,
//!     
//!     #[validate(length(min = 3, max = 50))]
//!     username: String,
//!     
//!     #[validate(async_unique(table = "users", column = "email"))]
//!     unique_email: String,
//! }
//! ```
//!
//! ## Validation Rules
//!
//! - `email` - Validates email format
//! - `length(min = X, max = Y)` - String length validation
//! - `range(min = X, max = Y)` - Numeric range validation
//! - `regex = "..."` - Regex pattern validation
//! - `url` - URL format validation
//! - `required` - Non-empty string/option validation
//! - `async_unique(table, column)` - Database uniqueness check
//! - `async_exists(table, column)` - Database existence check
//! - `async_api(endpoint)` - External API validation
//!
//! ## Error Format
//!
//! Validation errors return a 422 Unprocessable Entity with JSON:
//!
//! ```json
//! {
//!   "error": {
//!     "type": "validation_error",
//!     "message": "Validation failed",
//!     "fields": [
//!       {"field": "email", "code": "email", "message": "Invalid email format"},
//!       {"field": "age", "code": "range", "message": "Value must be between 18 and 120"}
//!     ]
//!   }
//! }
//! ```

pub mod custom;
mod error;
mod validate;

/// V2 validation engine with async support.
///
/// This module provides a custom validation engine that replaces the external
/// `validator` dependency and adds support for async validation operations.
pub mod v2;

pub use error::{FieldError, ValidationError};
pub use validate::Validate;

// Re-export the derive macro from validator (wrapped)
// In a full implementation, we'd create our own proc-macro
// For now, we use validator's derive with our own trait
pub use validator::Validate as ValidatorValidate;

// Re-export the v2 Validate derive macro
pub use rustapi_macros::Validate as DeriveValidate;

/// Prelude module for validation
pub mod prelude {
    pub use crate::error::{FieldError, ValidationError};
    pub use crate::validate::Validate;
    pub use validator::Validate as ValidatorValidate;

    // Re-export v2 prelude
    pub use crate::v2::prelude::*;

    // Re-export derive macro
    pub use rustapi_macros::Validate as DeriveValidate;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_to_json() {
        let error = ValidationError::new(vec![
            FieldError::new("email", "email", "Invalid email format"),
            FieldError::new("age", "range", "Value must be between 18 and 120"),
        ]);

        let json = serde_json::to_string_pretty(&error).unwrap();
        assert!(json.contains("validation_error"));
        assert!(json.contains("email"));
        assert!(json.contains("age"));
    }
}
