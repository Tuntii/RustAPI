use rustapi_rs::prelude::*;

#[derive(Serialize, Deserialize, Schema)]
struct User {
    id: i64,
    name: String,
}

// Handler with typed error responses
#[rustapi_rs::get("/typed-error-users/{id}")]
#[rustapi_rs::errors(404 = "User not found", 403 = "Forbidden")]
async fn get_user_typed(Path(_id): Path<i64>) -> Result<Json<User>> {
    Ok(Json(User {
        id: 1,
        name: "Alice".to_string(),
    }))
}

// Handler with a single error response
#[rustapi_rs::delete("/typed-error-users/{id}")]
#[rustapi_rs::errors(404 = "User not found")]
async fn delete_user_typed(Path(_id): Path<i64>) -> Result<()> {
    Ok(())
}

// Handler with many error responses
#[rustapi_rs::post("/typed-error-users")]
#[rustapi_rs::errors(400 = "Invalid input", 409 = "Email already exists", 422 = "Validation failed")]
async fn create_user_typed(Json(_body): Json<User>) -> Result<Created<User>> {
    Ok(Created(User {
        id: 2,
        name: "Bob".to_string(),
    }))
}

#[test]
fn test_typed_errors_appear_in_openapi() {
    let app = RustApi::auto();
    let spec = app.openapi_spec();

    // Check /typed-error-users/{id} GET endpoint
    let path_item = spec
        .paths
        .get("/typed-error-users/{id}")
        .expect("OpenAPI should contain /typed-error-users/{id}");

    let get_op = path_item.get.as_ref().expect("GET operation should exist");

    // Should have 404 response
    let resp_404 = get_op
        .responses
        .get("404")
        .expect("Should have 404 response");
    assert_eq!(resp_404.description, "User not found");
    assert!(
        resp_404.content.contains_key("application/json"),
        "404 response should have application/json content"
    );

    // Should have 403 response
    let resp_403 = get_op
        .responses
        .get("403")
        .expect("Should have 403 response");
    assert_eq!(resp_403.description, "Forbidden");

    // Check DELETE endpoint
    let delete_op = path_item
        .delete
        .as_ref()
        .expect("DELETE operation should exist");
    let del_404 = delete_op
        .responses
        .get("404")
        .expect("DELETE should have 404 response");
    assert_eq!(del_404.description, "User not found");
}

#[test]
fn test_typed_errors_post_endpoint() {
    let app = RustApi::auto();
    let spec = app.openapi_spec();

    let path_item = spec
        .paths
        .get("/typed-error-users")
        .expect("OpenAPI should contain /typed-error-users");

    let post_op = path_item
        .post
        .as_ref()
        .expect("POST operation should exist");

    // Should have 400, 409, 422 responses
    let resp_400 = post_op
        .responses
        .get("400")
        .expect("Should have 400 response");
    assert_eq!(resp_400.description, "Invalid input");

    let resp_409 = post_op
        .responses
        .get("409")
        .expect("Should have 409 response");
    assert_eq!(resp_409.description, "Email already exists");

    let resp_422 = post_op
        .responses
        .get("422")
        .expect("Should have 422 response");
    assert_eq!(resp_422.description, "Validation failed");

    // Each error response should reference ErrorSchema
    for (code, resp) in &post_op.responses {
        if code.starts_with('4') || code.starts_with('5') {
            let content = resp.content.get("application/json");
            assert!(
                content.is_some(),
                "Error response {} should have application/json content",
                code
            );
        }
    }
}

// Test that correct extractor ordering compiles fine
#[rustapi_rs::post("/extractor-order-ok")]
async fn correct_order(
    Path(_id): Path<i64>,
    Json(_body): Json<User>, // Body-consuming LAST = correct
) -> Result<Json<User>> {
    Ok(Json(User {
        id: 1,
        name: "test".to_string(),
    }))
}

#[test]
fn test_correct_extractor_order_compiles() {
    // If this test compiles and runs, it means correct extractor ordering passes
    let routes = rustapi_rs::collect_auto_routes();
    assert!(
        routes
            .iter()
            .any(|r| r.path() == "/extractor-order-ok"),
        "Route with correct extractor order should exist"
    );
}
