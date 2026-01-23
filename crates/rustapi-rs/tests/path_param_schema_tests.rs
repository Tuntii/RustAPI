//! Tests for automatic Path<T> parameter schema detection in OpenAPI.
//!
//! These tests verify that the macro correctly infers OpenAPI schema types
//! from Rust types used in Path<T> extractors.

use rustapi_rs::collect_auto_routes;
use rustapi_rs::prelude::*;
use uuid::Uuid;

// =============================================================================
// Test Handlers with Various Path Parameter Types
// =============================================================================

/// Handler with UUID path parameter - should auto-detect as "uuid" schema
#[rustapi_rs::get("/items/{item_id}")]
async fn get_item_uuid(Path(item_id): Path<Uuid>) -> String {
    format!("Item: {}", item_id)
}

/// Handler with i64 path parameter - should auto-detect as "int64" schema
#[rustapi_rs::get("/users/{user_id}")]
async fn get_user_i64(Path(user_id): Path<i64>) -> String {
    format!("User: {}", user_id)
}

/// Handler with i32 path parameter - should auto-detect as "int32" schema
#[rustapi_rs::get("/posts/{post_id}")]
async fn get_post_i32(Path(post_id): Path<i32>) -> String {
    format!("Post: {}", post_id)
}

/// Handler with String path parameter - should auto-detect as "string" schema
#[rustapi_rs::get("/slugs/{slug}")]
async fn get_by_slug(Path(slug): Path<String>) -> String {
    format!("Slug: {}", slug)
}

/// Handler with bool path parameter - should auto-detect as "boolean" schema
#[rustapi_rs::get("/flags/{flag}")]
async fn get_flag(Path(flag): Path<bool>) -> String {
    format!("Flag: {}", flag)
}

/// Handler with f64 path parameter - should auto-detect as "number" schema
#[rustapi_rs::get("/scores/{score}")]
async fn get_score(Path(score): Path<f64>) -> String {
    format!("Score: {}", score)
}

/// Handler with manual #[param] override - should use manual schema, not auto
#[rustapi_rs::get("/custom/{id}")]
#[rustapi_rs::param(id, schema = "string")]
async fn get_custom_override(Path(id): Path<i64>) -> String {
    format!("Custom: {}", id)
}

// =============================================================================
// Tests
// =============================================================================

#[test]
fn test_uuid_param_auto_detection() {
    let routes = collect_auto_routes();
    let route = routes
        .iter()
        .find(|r| r.path() == "/items/{item_id}")
        .expect("Route /items/{item_id} should exist");

    let schema = route.param_schemas().get("item_id");
    assert_eq!(
        schema,
        Some(&"uuid".to_string()),
        "Path<Uuid> should auto-detect as 'uuid' schema"
    );
}

#[test]
fn test_i64_param_auto_detection() {
    let routes = collect_auto_routes();
    let route = routes
        .iter()
        .find(|r| r.path() == "/users/{user_id}")
        .expect("Route /users/{user_id} should exist");

    let schema = route.param_schemas().get("user_id");
    assert_eq!(
        schema,
        Some(&"int64".to_string()),
        "Path<i64> should auto-detect as 'int64' schema"
    );
}

#[test]
fn test_i32_param_auto_detection() {
    let routes = collect_auto_routes();
    let route = routes
        .iter()
        .find(|r| r.path() == "/posts/{post_id}")
        .expect("Route /posts/{post_id} should exist");

    let schema = route.param_schemas().get("post_id");
    assert_eq!(
        schema,
        Some(&"int32".to_string()),
        "Path<i32> should auto-detect as 'int32' schema"
    );
}

#[test]
fn test_string_param_auto_detection() {
    let routes = collect_auto_routes();
    let route = routes
        .iter()
        .find(|r| r.path() == "/slugs/{slug}")
        .expect("Route /slugs/{slug} should exist");

    let schema = route.param_schemas().get("slug");
    assert_eq!(
        schema,
        Some(&"string".to_string()),
        "Path<String> should auto-detect as 'string' schema"
    );
}

#[test]
fn test_bool_param_auto_detection() {
    let routes = collect_auto_routes();
    let route = routes
        .iter()
        .find(|r| r.path() == "/flags/{flag}")
        .expect("Route /flags/{flag} should exist");

    let schema = route.param_schemas().get("flag");
    assert_eq!(
        schema,
        Some(&"boolean".to_string()),
        "Path<bool> should auto-detect as 'boolean' schema"
    );
}

#[test]
fn test_f64_param_auto_detection() {
    let routes = collect_auto_routes();
    let route = routes
        .iter()
        .find(|r| r.path() == "/scores/{score}")
        .expect("Route /scores/{score} should exist");

    let schema = route.param_schemas().get("score");
    assert_eq!(
        schema,
        Some(&"number".to_string()),
        "Path<f64> should auto-detect as 'number' schema"
    );
}

#[test]
fn test_manual_param_override() {
    let routes = collect_auto_routes();
    let route = routes
        .iter()
        .find(|r| r.path() == "/custom/{id}")
        .expect("Route /custom/{id} should exist");

    let schema = route.param_schemas().get("id");
    assert_eq!(
        schema,
        Some(&"string".to_string()),
        "Manual #[param] should override auto-detection (i64 -> string)"
    );
}

#[test]
fn test_openapi_spec_has_correct_uuid_param() {
    let app = RustApi::auto();
    let spec = app.openapi_spec();

    // Convert spec to JSON using built-in method
    let json = spec.to_json();

    // Navigate to /items/{item_id} GET parameters
    let item_id_schema = json
        .pointer("/paths/~1items~1{item_id}/get/parameters/0/schema")
        .expect("Should have parameter schema");

    assert_eq!(
        item_id_schema.get("type"),
        Some(&serde_json::json!("string")),
        "UUID param type should be string"
    );
    assert_eq!(
        item_id_schema.get("format"),
        Some(&serde_json::json!("uuid")),
        "UUID param format should be uuid"
    );
}

#[test]
fn test_openapi_spec_has_correct_int64_param() {
    let app = RustApi::auto();
    let spec = app.openapi_spec();

    let json = spec.to_json();

    let user_id_schema = json
        .pointer("/paths/~1users~1{user_id}/get/parameters/0/schema")
        .expect("Should have parameter schema");

    assert_eq!(
        user_id_schema.get("type"),
        Some(&serde_json::json!("integer")),
        "i64 param type should be integer"
    );
    assert_eq!(
        user_id_schema.get("format"),
        Some(&serde_json::json!("int64")),
        "i64 param format should be int64"
    );
}
