use crate::error::{ApiError, FieldError};

/// Unified validation trait for synchronous validation
///
/// This trait allows uniform access to both `validator` (external) and
/// `rustapi_validate::v2` (internal) validation engines.
pub trait Validatable {
    /// Perform synchronous validation
    fn do_validate(&self) -> Result<(), ApiError>;
}

// Blanket implementation for types implementing the external validator::Validate trait
#[cfg(feature = "legacy-validator")]
impl<T: validator::Validate> Validatable for T {
    fn do_validate(&self) -> Result<(), ApiError> {
        match validator::Validate::validate(self) {
            Ok(_) => Ok(()),
            Err(e) => Err(convert_validator_errors(e)),
        }
    }
}

/// Helper to convert validator::ValidationErrors to rustapi_core::error::ApiError
#[cfg(feature = "legacy-validator")]
pub fn convert_validator_errors(errors: validator::ValidationErrors) -> ApiError {
    let field_errors =
        errors
            .field_errors()
            .iter()
            .flat_map(|(field, errs)| {
                let field_name = field.to_string();
                errs.iter().map(move |e| FieldError {
                    field: field_name.clone(),
                    code: e.code.to_string(),
                    message: e.message.clone().map(|m| m.to_string()).unwrap_or_else(|| {
                        format!("Validation failed for field '{}'", &field_name)
                    }),
                })
            })
            .collect();
    ApiError::validation(field_errors)
}

/// Helper to convert rustapi_validate::v2::ValidationErrors to rustapi_core::error::ApiError
pub fn convert_v2_errors(errors: rustapi_validate::v2::ValidationErrors) -> ApiError {
    let field_errors = errors
        .fields
        .iter()
        .flat_map(|(field, errs)| {
            let field_name = field.to_string();
            errs.iter().map(move |e| FieldError {
                field: field_name.clone(),
                code: e.code.to_string(),
                message: e.message.clone(),
            })
        })
        .collect();
    ApiError::validation(field_errors)
}
