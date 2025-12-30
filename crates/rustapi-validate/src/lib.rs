//! # RustAPI Validation
//!
//! Validation system for RustAPI framework. Provides declarative validation
//! on structs using the `#[derive(Validate)]` macro from the `validator` crate.
//!
//! ## Example
//!
//! ```rust,ignore
//! use rustapi_validate::prelude::*;
//!
//! #[derive(ValidatorValidate)]
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
//! ## Validation Rules
//!
//! - `email` - Validates email format
//! - `length(min = X, max = Y)` - String length validation
//! - `range(min = X, max = Y)` - Numeric range validation
//! - `regex = "..."` - Regex pattern validation
//! - `non_empty` - Non-empty string/collection validation
//! - `nested` - Validates nested structs
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

mod error;
mod validate;

pub use error::{FieldError, ValidationError};
pub use validate::Validate;

// Re-export the derive macro from validator (wrapped)
// Users should use `validator::Validate` derive macro with our trait
pub use validator::Validate as ValidatorValidate;

/// Prelude module for validation
pub mod prelude {
    pub use crate::error::{FieldError, ValidationError};
    pub use crate::validate::Validate;
    pub use validator::Validate as ValidatorValidate;
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
