use rustapi_rs::collect_auto_routes;
use rustapi_rs::prelude::*;
use rustapi_rs::{get, post};
use serde::Deserialize;

// Standard handler
#[get("/test-auto-rs")]
async fn auto_handler_rs() -> &'static str {
    "auto-rs"
}

// Another handler
#[post("/test-auto-rs-post")]
async fn auto_handler_rs_post() -> &'static str {
    "auto-rs-post"
}

#[test]
fn test_auto_registration_rs() {
    // Collect routes
    let routes = collect_auto_routes();

    // Filter to find our specific routes
    let found_auto = routes
        .iter()
        .any(|r| r.path() == "/test-auto-rs" && r.method() == "GET");
    let found_auto_post = routes
        .iter()
        .any(|r| r.path() == "/test-auto-rs-post" && r.method() == "POST");

    assert!(found_auto, "Should find /test-auto-rs GET route");
    assert!(found_auto_post, "Should find /test-auto-rs-post POST route");

    println!("Found {} routes", routes.len());
}

#[get("/same-path")]
async fn same_path_get() -> &'static str {
    "get"
}

#[post("/same-path")]
async fn same_path_post() -> &'static str {
    "post"
}

#[derive(Debug, Clone, Serialize, Schema)]
struct AutoSchemaType {
    id: i64,
}

#[get("/schema")]
async fn schema_handler() -> Json<AutoSchemaType> {
    Json(AutoSchemaType { id: 1 })
}

#[get("/users/{id}")]
async fn get_user(Path(_id): Path<i64>) -> &'static str {
    "ok"
}

#[derive(Debug, Clone, Deserialize, Schema)]
struct Pagination {
    page: Option<u32>,
    page_size: Option<u32>,
}

#[get("/query")]
async fn query_handler(Query(p): Query<Pagination>) -> &'static str {
    let _ = (&p.page, &p.page_size);
    "ok"
}

#[test]
fn test_auto_groups_methods_by_path() {
    let app = RustApi::auto();
    let router = app.into_router();

    let registered = router
        .registered_routes()
        .get("/same-path")
        .expect("/same-path should be registered");

    assert!(
        registered.methods.iter().any(|m| m.as_str() == "GET"),
        "GET should be registered"
    );
    assert!(
        registered.methods.iter().any(|m| m.as_str() == "POST"),
        "POST should be registered"
    );
}

#[test]
fn test_auto_registers_schemas() {
    let app = RustApi::auto();
    let spec = app.openapi_spec();

    use rustapi_openapi::schema::RustApiSchema;
    let name = <AutoSchemaType as RustApiSchema>::component_name().unwrap();

    let components = spec.components.as_ref().expect("Components should exist");
    assert!(
        components.schemas.contains_key(name),
        "AutoSchemaType should be registered into OpenAPI components"
    );
}

#[test]
fn test_openapi_includes_path_params() {
    let app = RustApi::auto();
    let spec = app.openapi_spec();

    let path_item = spec
        .paths
        .get("/users/{id}")
        .expect("OpenAPI should contain /users/{id}");

    let op = path_item.get.as_ref().expect("GET operation should exist");
    let params = &op.parameters;

    assert!(
        params
            .iter()
            .any(|p| p.location == "path" && p.name == "id" && p.required),
        "OpenAPI should include required path parameter 'id'"
    );
}

#[test]
fn test_openapi_includes_query_params() {
    let app = RustApi::auto();
    let spec = app.openapi_spec();

    let path_item = spec
        .paths
        .get("/query")
        .expect("OpenAPI should contain /query");

    let op = path_item.get.as_ref().expect("GET operation should exist");
    let params = &op.parameters;

    assert!(
        params
            .iter()
            .any(|p| p.location == "query" && p.name == "page"),
        "OpenAPI should include query parameter 'page'"
    );
    assert!(
        params
            .iter()
            .any(|p| p.location == "query" && p.name == "page_size"),
        "OpenAPI should include query parameter 'page_size'"
    );
}
