use rustapi_macros::Validate;
use rustapi_validate::v2::{
    AsyncApiRule, AsyncExistsRule, AsyncUniqueRule, EmailRule, LengthRule, Validate,
    ValidationErrors,
};

#[derive(Validate)]
struct CustomMessageTest {
    #[validate(email(message = "Invalid email format custom"))]
    email: String,

    #[validate(length(min = 5, message = "Too short custom"))]
    username: String,
}

#[test]
fn test_macro_custom_messages_sync() {
    let t = CustomMessageTest {
        email: "not-an-email".to_string(),
        username: "tiny".to_string(),
    };

    let result = t.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();

    let email_errs = errors.get("email").unwrap();
    assert_eq!(
        email_errs[0].message,
        "Invalid email format custom".to_string()
    );

    let username_errs = errors.get("username").unwrap();
    assert_eq!(username_errs[0].message, "Too short custom".to_string());
}

#[test]
fn test_builder_pattern_sync() {
    let rule = EmailRule::new().with_message("Builder custom message");
    let result =
        rustapi_validate::v2::ValidationRule::validate(&rule, &"invalid-email".to_string());

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().message,
        "Builder custom message".to_string()
    );
}

#[tokio::test]
async fn test_builder_pattern_async() {
    // We don't need a real context just to check if the message is preserved in the rule struct
    // effectively, we are testing the builder configuration.

    let rule = AsyncUniqueRule::new("users", "email").with_message("Async unique custom");
    assert_eq!(rule.message, Some("Async unique custom".to_string()));

    let rule = AsyncExistsRule::new("users", "id").with_message("Async exists custom");
    assert_eq!(rule.message, Some("Async exists custom".to_string()));

    let rule = AsyncApiRule::new("https://example.com").with_message("Async api custom");
    assert_eq!(rule.message, Some("Async api custom".to_string()));
}
