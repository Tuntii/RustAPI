use rustapi_macros::Validate;
use rustapi_validate::v2::{Validate, ValidationContextBuilder, ValidationErrors};

#[derive(Validate)]
struct I18nTest {
    #[validate(email)]
    email: String,

    #[validate(length(min = 5))]
    username: String,
}

// Helper to assert localization
fn assert_localized(errors: &ValidationErrors, field: &str, locale: Option<&str>, expected: &str) {
    let errs = errors.get(field).unwrap();
    let err = &errs[0];
    let msg = err.interpolate_with_locale(locale);
    assert_eq!(msg, expected, "Failed for locale {:?}", locale);
}

#[test]
fn test_default_locale_english() {
    // rust-i18n defaults to the system locale or fallback.
    // We should set it explicitly to ensure consistent tests if possible,
    // or rely on fallback to "en".

    // Force set locale to en for this thread/test context would be ideal but rust-i18n sets global.
    rust_i18n::set_locale("en");

    let valid = I18nTest {
        email: "invalid".to_string(),
        username: "tiny".to_string(),
    };

    let result = valid.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();

    // Check with default locale (None) which should use global "en"
    assert_localized(&errors, "email", None, "Invalid email format");
    // "Length must be at least 5 characters"
    assert_localized(
        &errors,
        "username",
        None,
        "Length must be at least 5 characters",
    );
}

#[test]
fn test_explicit_locale_turkish() {
    let valid = I18nTest {
        email: "invalid".to_string(),
        username: "tiny".to_string(),
    };

    let result = valid.validate();
    let errors = result.unwrap_err();

    // Check with explicit locale "tr"
    assert_localized(&errors, "email", Some("tr"), "Geçersiz e-posta formatı");
    // "Uzunluk en az 5 karakter olmalıdır"
    assert_localized(
        &errors,
        "username",
        Some("tr"),
        "Uzunluk en az 5 karakter olmalıdır",
    );
}

#[test]
fn test_context_locale() {
    // This requires us to pass the context locale to the error interpolation somehow.
    // But Validation::validate() returns ValidationErrors which are locale-agnostic until interpolation.
    // The API layer would get the locale from the context and pass it to into_api_error_with_locale.

    let ctx = ValidationContextBuilder::new().locale("tr").build();
    let locale = ctx.locale();

    let valid = I18nTest {
        email: "invalid".to_string(),
        username: "tiny".to_string(),
    };

    let result = valid.validate();
    let errors = result.unwrap_err();

    let api_error = errors.to_api_error_with_locale(locale);
    let email_err = api_error
        .error
        .fields
        .iter()
        .find(|f| f.field == "email")
        .unwrap();

    assert_eq!(email_err.message, "Geçersiz e-posta formatı");
}

#[test]
fn test_fallback_to_english() {
    let valid = I18nTest {
        email: "invalid".to_string(),
        username: "tiny".to_string(),
    };

    let result = valid.validate();
    let errors = result.unwrap_err();

    // Check with unsupported locale "fr" -> should fallback to default (en)
    // Note: rust-i18n behavior depends on configuration.
    // If fallback is enabled, it should work.

    rust_i18n::set_locale("en"); // Ensure default is en

    // localize with "fr"
    let errs = errors.get("email").unwrap();
    let msg = errs[0].interpolate_with_locale(Some("fr"));

    // If strict, it might return key or english. `rust-i18n` usually falls back to default.
    // Let's assume fallback to "en".
    assert_eq!(msg, "Invalid email format");
}
