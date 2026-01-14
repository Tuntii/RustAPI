//! # RustAPI Validation Engine v2
//!
//! A custom validation engine with async support, replacing the external `validator` dependency.
//!
//! ## Features
//!
//! - Sync and async validation support
//! - Database uniqueness/existence checks
//! - External API validation
//! - Custom error messages with interpolation
//! - Validation groups (Create, Update, Custom)
//! - Serializable validation rules
//!
//! ## Example
//!
//! ```rust,ignore
//! use rustapi_validate::v2::prelude::*;
//!
//! struct CreateUser {
//!     email: String,
//!     username: String,
//! }
//!
//! impl Validate for CreateUser {
//!     fn validate(&self) -> Result<(), ValidationErrors> {
//!         let mut errors = ValidationErrors::new();
//!         
//!         if let Err(e) = EmailRule::default().validate(&self.email) {
//!             errors.add("email", e);
//!         }
//!         
//!         if let Err(e) = LengthRule::new(3, 50).validate(&self.username) {
//!             errors.add("username", e);
//!         }
//!         
//!         errors.into_result()
//!     }
//! }
//! ```

mod context;
mod error;
mod group;
mod rules;
mod traits;

#[cfg(test)]
mod tests;

pub use context::{DatabaseValidator, HttpValidator, ValidationContext, ValidationContextBuilder};
pub use error::{RuleError, ValidationErrors};
pub use group::{GroupedRule, GroupedRules, ValidationGroup};
pub use rules::*;
pub use traits::{AsyncValidate, AsyncValidationRule, SerializableRule, Validate, ValidationRule};

/// Prelude module for v2 validation
pub mod prelude {
    pub use super::context::{
        DatabaseValidator, HttpValidator, ValidationContext, ValidationContextBuilder,
    };
    pub use super::error::{RuleError, ValidationErrors};
    pub use super::group::{GroupedRule, GroupedRules, ValidationGroup};
    pub use super::rules::*;
    pub use super::traits::{
        AsyncValidate, AsyncValidationRule, SerializableRule, Validate, ValidationRule,
    };
}
