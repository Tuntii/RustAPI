use rustapi_macros::Validate;
use rustapi_validate::v2::prelude::*;
use serde::Serialize;

#[derive(Debug, Validate, Serialize)]
struct RichRulesDto {
    #[validate(credit_card)]
    cc: String,

    #[validate(ip)]
    any_ip: String,

    #[validate(ip(v4))]
    ipv4: String,

    #[validate(ip(v6))]
    ipv6: String,

    #[validate(phone)]
    phone: String,

    #[validate(contains(needle = "rust"))]
    about: String,
}

#[test]
fn test_rich_rules_valid() {
    let dto = RichRulesDto {
        cc: "4532015112830366".to_string(), // Valid Visa test card (Luhn-valid)
        any_ip: "127.0.0.1".to_string(),
        ipv4: "192.168.1.1".to_string(),
        ipv6: "2001:db8::1".to_string(),
        phone: "+14155552671".to_string(),
        about: "I love rust programming".to_string(),
    };

    assert!(dto.validate().is_ok());
}

#[test]
fn test_rich_rules_invalid() {
    let dto = RichRulesDto {
        cc: "1234567890123456".to_string(), // Invalid Luhn
        any_ip: "not-an-ip".to_string(),
        ipv4: "2001:db8::1".to_string(),    // IPv6 in IPv4 field
        ipv6: "192.168.1.1".to_string(),    // IPv4 in IPv6 field
        phone: "123".to_string(),           // Invalid phone
        about: "I love python".to_string(), // Missing "rust"
    };

    let result = dto.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();

    assert!(errors.get("cc").is_some());
    assert!(errors.get("any_ip").is_some());
    assert!(errors.get("ipv4").is_some());
    assert!(errors.get("ipv6").is_some());
    assert!(errors.get("phone").is_some());
    assert!(errors.get("about").is_some());
}

// Note: Custom message with nested syntax
#[derive(Debug, Validate)]
struct CustomMessageDto {
    #[validate(credit_card(message = "Invalid CC"))]
    cc: String,

    #[validate(ip(v4, message = "Must be IPv4"))]
    ipv4: String,
}

#[test]
fn test_custom_messages() {
    let dto = CustomMessageDto {
        cc: "123".to_string(),
        ipv4: "invalid".to_string(),
    };

    let errors = dto.validate().unwrap_err();

    assert_eq!(errors.get("cc").unwrap()[0].message, "Invalid CC");
    assert_eq!(errors.get("ipv4").unwrap()[0].message, "Must be IPv4");
}
