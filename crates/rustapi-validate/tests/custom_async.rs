use rustapi_macros::Validate;
use rustapi_validate::v2::{prelude::*, RuleError, ValidationContext};

// Custom async validator function
// Signature must be: async fn(&T, &ValidationContext) -> Result<(), RuleError>
async fn validate_username_available(
    username: &String,
    _ctx: &ValidationContext,
) -> Result<(), RuleError> {
    // Simulate async DB check
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    if username == "taken" {
        Err(RuleError::new("custom_check", "Username is taken"))
    } else {
        Ok(())
    }
}

// Another custom validator with specific error
async fn validate_complex_logic(value: &String, _ctx: &ValidationContext) -> Result<(), RuleError> {
    if value.starts_with("fail") {
        Err(RuleError::new("complex", "Complex validation failed"))
    } else {
        Ok(())
    }
}

#[derive(Debug, Validate)]
struct UserSignup {
    #[validate(custom_async = "validate_username_available")]
    username: String,

    #[validate(custom_async(
        function = "validate_complex_logic",
        message = "Custom message override"
    ))]
    bio: String,
}

#[tokio::test]
async fn test_custom_async_validation_success() {
    let user = UserSignup {
        username: "available".to_string(),
        bio: "valid bio".to_string(),
    };

    let ctx = ValidationContext::builder().build();
    let result = user.validate_async(&ctx).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_custom_async_validation_fail_logic() {
    let user = UserSignup {
        username: "taken".to_string(),
        bio: "valid bio".to_string(),
    };

    let ctx = ValidationContext::builder().build();
    let result = user.validate_async(&ctx).await;

    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("username").is_some());
    // Should get original error message
    assert_eq!(
        errors.get("username").unwrap()[0].message,
        "Username is taken"
    );
}

#[tokio::test]
async fn test_custom_async_validation_message_override() {
    let user = UserSignup {
        username: "available".to_string(),
        bio: "fail this".to_string(),
    };

    let ctx = ValidationContext::builder().build();
    let result = user.validate_async(&ctx).await;

    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("bio").is_some());
    // Should get overridden message
    assert_eq!(
        errors.get("bio").unwrap()[0].message,
        "Custom message override"
    );
}
