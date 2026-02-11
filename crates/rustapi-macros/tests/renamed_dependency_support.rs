use api as rustapi_alias;
use rustapi_alias::prelude::*;

#[rustapi_alias::get("/macro-alias-get")]
async fn alias_get() -> &'static str {
    "ok"
}

#[rustapi_alias::post("/macro-alias-post")]
async fn alias_post() -> &'static str {
    "ok"
}

#[rustapi_alias::schema]
#[derive(Schema)]
#[allow(dead_code)]
struct AliasSchema {
    id: i64,
}

#[derive(TypedPath, Serialize, Deserialize)]
#[typed_path("/macro-alias/{id}")]
struct AliasPath {
    id: i64,
}

#[test]
fn renamed_dependency_supports_route_macros() {
    let routes = rustapi_alias::collect_auto_routes();
    assert!(
        routes
            .iter()
            .any(|r| r.path() == "/macro-alias-get" && r.method() == "GET"),
        "GET route should be discovered via #[api::get]"
    );
    assert!(
        routes
            .iter()
            .any(|r| r.path() == "/macro-alias-post" && r.method() == "POST"),
        "POST route should be discovered via #[api::post]"
    );
}

#[test]
fn renamed_dependency_supports_schema_and_typed_path_macros() {
    let uri = AliasPath { id: 7 }.to_uri();
    assert_eq!(uri, "/macro-alias/7");

    let app = rustapi_alias::RustApi::auto();
    let spec = app.openapi_spec();
    let schemas = &spec
        .components
        .as_ref()
        .expect("components should exist")
        .schemas;
    assert!(
        schemas.contains_key("AliasSchema"),
        "schema should be registered via #[api::schema]"
    );
}
