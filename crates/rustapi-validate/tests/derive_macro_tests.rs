//! Integration tests for the Validate derive macro.
//!
//! These tests verify that the derive macro correctly generates
//! Validate and AsyncValidate implementations.

use async_trait::async_trait;
use rustapi_validate::v2::{AsyncValidate, DatabaseValidator, Validate, ValidationContextBuilder};
use rustapi_validate::DeriveValidate;

// Test struct using the derive macro with sync validation rules
#[derive(DeriveValidate)]
struct CreateUser {
    #[validate(email(message = "Invalid email format"))]
    email: String,

    #[validate(length(min = 3, max = 50))]
    username: String,

    #[validate(range(min = 18, max = 120))]
    age: u8,
}

#[test]
fn derive_validate_sync_valid() {
    let user = CreateUser {
        email: "test@example.com".to_string(),
        username: "johndoe".to_string(),
        age: 25,
    };

    let result = user.validate();
    assert!(result.is_ok());
}

#[test]
fn derive_validate_sync_invalid_email() {
    let user = CreateUser {
        email: "invalid-email".to_string(),
        username: "johndoe".to_string(),
        age: 25,
    };

    let result = user.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("email").is_some());
    assert_eq!(
        errors.get("email").unwrap()[0].message,
        "Invalid email format"
    );
}

#[test]
fn derive_validate_sync_invalid_length() {
    let user = CreateUser {
        email: "test@example.com".to_string(),
        username: "ab".to_string(), // Too short
        age: 25,
    };

    let result = user.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("username").is_some());
}

#[test]
fn derive_validate_sync_invalid_range() {
    let user = CreateUser {
        email: "test@example.com".to_string(),
        username: "johndoe".to_string(),
        age: 15, // Too young
    };

    let result = user.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("age").is_some());
}

#[test]
fn derive_validate_sync_multiple_errors() {
    let user = CreateUser {
        email: "invalid".to_string(),
        username: "ab".to_string(),
        age: 15,
    };

    let result = user.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("email").is_some());
    assert!(errors.get("username").is_some());
    assert!(errors.get("age").is_some());
}

// Test struct with URL and required validation
#[derive(DeriveValidate)]
struct Website {
    #[validate(required)]
    name: String,

    #[validate(url)]
    homepage: String,
}

#[test]
fn derive_validate_url_valid() {
    let site = Website {
        name: "My Site".to_string(),
        homepage: "https://example.com".to_string(),
    };

    assert!(site.validate().is_ok());
}

#[test]
fn derive_validate_url_invalid() {
    let site = Website {
        name: "My Site".to_string(),
        homepage: "not-a-url".to_string(),
    };

    let result = site.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("homepage").is_some());
}

#[test]
fn derive_validate_required_empty() {
    let site = Website {
        name: "   ".to_string(), // Whitespace only
        homepage: "https://example.com".to_string(),
    };

    let result = site.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("name").is_some());
}

// Test struct with regex validation
#[derive(DeriveValidate)]
struct PhoneNumber {
    #[validate(regex(pattern = r"^\d{3}-\d{4}$", message = "Invalid phone format"))]
    number: String,
}

#[test]
fn derive_validate_regex_valid() {
    let phone = PhoneNumber {
        number: "123-4567".to_string(),
    };

    assert!(phone.validate().is_ok());
}

#[test]
fn derive_validate_regex_invalid() {
    let phone = PhoneNumber {
        number: "1234567".to_string(),
    };

    let result = phone.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("number").is_some());
    assert_eq!(
        errors.get("number").unwrap()[0].message,
        "Invalid phone format"
    );
}

// Test async validation with derive macro
#[derive(DeriveValidate)]
struct AsyncUser {
    #[validate(email)]
    email: String,

    #[validate(async_unique(table = "users", column = "email", message = "Email already taken"))]
    unique_email: String,
}

// Mock database validator for async tests
struct MockDbValidator {
    taken_emails: Vec<String>,
}

#[async_trait]
impl DatabaseValidator for MockDbValidator {
    async fn exists(&self, _table: &str, _column: &str, _value: &str) -> Result<bool, String> {
        Ok(false)
    }

    async fn is_unique(&self, _table: &str, _column: &str, value: &str) -> Result<bool, String> {
        Ok(!self.taken_emails.contains(&value.to_string()))
    }

    async fn is_unique_except(
        &self,
        _table: &str,
        _column: &str,
        value: &str,
        _except_id: &str,
    ) -> Result<bool, String> {
        Ok(!self.taken_emails.contains(&value.to_string()))
    }
}

#[tokio::test]
async fn derive_validate_async_valid() {
    let user = AsyncUser {
        email: "test@example.com".to_string(),
        unique_email: "new@example.com".to_string(),
    };

    let db = MockDbValidator {
        taken_emails: vec!["taken@example.com".to_string()],
    };
    let ctx = ValidationContextBuilder::new().database(db).build();

    let result = user.validate_async(&ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn derive_validate_async_email_taken() {
    let user = AsyncUser {
        email: "test@example.com".to_string(),
        unique_email: "taken@example.com".to_string(),
    };

    let db = MockDbValidator {
        taken_emails: vec!["taken@example.com".to_string()],
    };
    let ctx = ValidationContextBuilder::new().database(db).build();

    let result = user.validate_async(&ctx).await;
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("unique_email").is_some());
    assert_eq!(
        errors.get("unique_email").unwrap()[0].message,
        "Email already taken"
    );
}

#[tokio::test]
async fn derive_validate_full_validation() {
    let user = AsyncUser {
        email: "test@example.com".to_string(),
        unique_email: "new@example.com".to_string(),
    };

    let db = MockDbValidator {
        taken_emails: vec![],
    };
    let ctx = ValidationContextBuilder::new().database(db).build();

    // Full validation runs both sync and async
    let result = user.validate_full(&ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn derive_validate_full_sync_fails() {
    let user = AsyncUser {
        email: "invalid-email".to_string(), // Sync validation fails
        unique_email: "new@example.com".to_string(),
    };

    let db = MockDbValidator {
        taken_emails: vec![],
    };
    let ctx = ValidationContextBuilder::new().database(db).build();

    // Full validation should fail on sync validation
    let result = user.validate_full(&ctx).await;
    assert!(result.is_err());
}
