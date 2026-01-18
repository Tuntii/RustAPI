use rustapi_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, TypedPath)]
#[typed_path("/users/{id}/details")]
struct UserDetailsPath {
    id: u64,
}

#[derive(Debug, Serialize, Deserialize, TypedPath)]
#[typed_path("/products/{category}/{item_id}")]
struct ProductPath {
    category: String,
    item_id: String,
}

#[test]
fn test_typed_path_constants() {
    assert_eq!(UserDetailsPath::PATH, "/users/{id}/details");
    assert_eq!(ProductPath::PATH, "/products/{category}/{item_id}");
}

#[test]
fn test_typed_path_to_uri() {
    let user_path = UserDetailsPath { id: 123 };
    assert_eq!(user_path.to_uri(), "/users/123/details");

    let product_path = ProductPath {
        category: "electronics".to_string(),
        item_id: "phone-1".to_string(),
    };
    assert_eq!(product_path.to_uri(), "/products/electronics/phone-1");
}

// Test compilation of handler signature
async fn _user_handler(Typed(params): Typed<UserDetailsPath>) -> String {
    format!("User ID: {}", params.id)
}

// Test router registration compilation
fn _register_routes() {
    let _app = RustApi::new().typed::<UserDetailsPath>(get(_user_handler));
}
