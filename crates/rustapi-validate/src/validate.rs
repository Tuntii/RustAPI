//! Validation trait and utilities.

use crate::error::ValidationError;

/// Trait for validatable types.
///
/// This trait wraps the `validator::Validate` trait and provides
/// a RustAPI-native interface for validation.
///
/// ## Example
///
/// ```rust,ignore
/// use rustapi_validate::prelude::*;
/// use validator::Validate as ValidatorValidate;
///
/// #[derive(ValidatorValidate)]
/// struct CreateUser {
///     #[validate(email)]
///     email: String,
///     
///     #[validate(length(min = 3, max = 50))]
///     username: String,
/// }
///
/// impl Validate for CreateUser {}
///
/// fn example() {
///     let user = CreateUser {
///         email: "invalid".to_string(),
///         username: "ab".to_string(),
///     };
///
///     match user.validate() {
///         Ok(()) => println!("Valid!"),
///         Err(e) => println!("Errors: {:?}", e.fields),
///     }
/// }
/// ```
pub trait Validate: validator::Validate {
    /// Validate the struct and return a `ValidationError` on failure.
    fn validate(&self) -> Result<(), ValidationError> {
        validator::Validate::validate(self)
            .map_err(ValidationError::from_validator_errors)
    }

    /// Validate and return the struct if valid, error otherwise.
    fn validated(self) -> Result<Self, ValidationError>
    where
        Self: Sized,
    {
        Validate::validate(&self)?;
        Ok(self)
    }
}

// Blanket implementation for all types that implement validator::Validate
impl<T: validator::Validate> Validate for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate as ValidatorValidate;

    #[derive(Debug, ValidatorValidate)]
    struct TestUser {
        #[validate(email)]
        email: String,
        #[validate(length(min = 3, max = 20))]
        username: String,
        #[validate(range(min = 18, max = 120))]
        age: u8,
    }

    #[test]
    fn valid_struct_passes() {
        let user = TestUser {
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            age: 25,
        };

        assert!(user.validate().is_ok());
    }

    #[test]
    fn invalid_email_fails() {
        let user = TestUser {
            email: "not-an-email".to_string(),
            username: "testuser".to_string(),
            age: 25,
        };

        let result = user.validate();
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.fields.iter().any(|f| f.field == "email"));
    }

    #[test]
    fn invalid_length_fails() {
        let user = TestUser {
            email: "test@example.com".to_string(),
            username: "ab".to_string(), // Too short
            age: 25,
        };

        let result = user.validate();
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.fields.iter().any(|f| f.field == "username" && f.code == "length"));
    }

    #[test]
    fn invalid_range_fails() {
        let user = TestUser {
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            age: 15, // Too young
        };

        let result = user.validate();
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.fields.iter().any(|f| f.field == "age" && f.code == "range"));
    }

    #[test]
    fn multiple_errors_collected() {
        let user = TestUser {
            email: "invalid".to_string(),
            username: "ab".to_string(),
            age: 150,
        };

        let result = user.validate();
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.fields.len() >= 3);
    }

    #[test]
    fn validated_returns_struct_on_success() {
        let user = TestUser {
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            age: 25,
        };

        let result = user.validated();
        assert!(result.is_ok());
        
        let validated_user = result.unwrap();
        assert_eq!(validated_user.email, "test@example.com");
    }
}
