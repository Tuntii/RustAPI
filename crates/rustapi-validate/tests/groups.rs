use rustapi_macros::Validate;
use rustapi_validate::v2::prelude::*;

#[derive(Validate)]
struct User {
    // Nested syntax: groups inside length(...)
    #[validate(length(min = 3, message = "Username too short", groups = ["Create", "Update"]))]
    username: String,

    // email is usually simple, but we can make it a list to support params
    #[validate(email(message = "Invalid email"))]
    email: String, // Always required (Default group)

    #[validate(length(min = 6, groups = ["Create"]))]
    password_hash: String, // Only checked on Create
}

#[test]
fn test_default_group_validation() {
    let user = User {
        username: "ab".to_string(),         // Invalid length
        email: "invalid-email".to_string(), // Invalid email
        password_hash: "123".to_string(),   // Invalid length
    };

    let res = user.validate();
    assert!(res.is_err());
    let errs = res.unwrap_err();

    // Email should fail (Default matches Default)
    assert!(
        errs.get("email").is_some(),
        "Email should fail on Default group"
    );

    // Username has explicit groups ["Create", "Update"].
    // Default context matches ONLY Default rules (after my fix to matches logic).
    // So Username should NOT run.
    assert!(
        errs.get("username").is_none(),
        "Username has explicit groups, should not run on Default context"
    );

    // Password has explicit group ["Create"], should not run on Default
    assert!(
        errs.get("password_hash").is_none(),
        "Password has explicit group Create, should not run on Default"
    );
}

#[test]
fn test_create_group_validation() {
    let user = User {
        username: "ab".to_string(),
        email: "invalid-email".to_string(),
        password_hash: "123".to_string(),
    };

    let res = user.validate_with_group(ValidationGroup::Create);
    assert!(res.is_err());
    let errs = res.unwrap_err();

    // Username should fail (Create matches Create)
    assert!(
        errs.get("username").is_some(),
        "Username should fail on Create group"
    );

    // Email should fail (Result matches Create? No. Default matches Default? Yes. But Default rule runs on Create context?)
    // Default Rule: groups=[].
    // Check: groups.any(|g| g.matches(Create)).
    // groups is empty. Macro logic: if groups.empty() { true }.
    // So Default rules run EVERYWHERE.
    assert!(
        errs.get("email").is_some(),
        "Email should fail on Create group"
    );

    // Password should fail (Create matches Create)
    assert!(
        errs.get("password_hash").is_some(),
        "Password should fail on Create group"
    );
}

#[test]
fn test_update_group_validation() {
    let user = User {
        username: "ab".to_string(),
        email: "invalid-email".to_string(),
        password_hash: "123".to_string(),
    };

    let res = user.validate_with_group(ValidationGroup::Update);
    assert!(res.is_err());
    let errs = res.unwrap_err();

    // Username check runs on Update (Update matches Update)
    assert!(
        errs.get("username").is_some(),
        "Username should fail on Update group"
    );

    // Email check runs on Update (Default rule runs everywhere)
    assert!(
        errs.get("email").is_some(),
        "Email should fail on Update group"
    );

    // Password check runs on Create only.
    // Create.matches(Update) -> False.
    assert!(
        errs.get("password_hash").is_none(),
        "Password should NOT fail on Update group"
    );
}

#[test]
fn test_custom_group_validation() {
    let user = User {
        username: "ab".to_string(),
        email: "invalid-email".to_string(),
        password_hash: "123".to_string(),
    };

    // Custom group "Admin"
    let res = user.validate_with_group(ValidationGroup::Custom("Admin".into()));
    assert!(res.is_err());
    let errs = res.unwrap_err();

    // Username: ["Create", "Update"] vs "Admin" -> No match
    assert!(
        errs.get("username").is_none(),
        "Username should NOT fail on Custom group"
    );

    // Email: Default runs everywhere
    assert!(
        errs.get("email").is_some(),
        "Email should fail on Custom group"
    );

    // Password: "Create" vs "Admin" -> No match
    assert!(
        errs.get("password_hash").is_none(),
        "Password should NOT fail on Custom group"
    );
}
