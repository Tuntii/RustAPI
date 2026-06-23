use super::RustApi;
use crate::extract::{FromRequestParts, State};
use crate::path_params::PathParams;
use crate::request::Request;
use crate::router::{get, post, Router};
use bytes::Bytes;
use http::Method;
use proptest::prelude::*;

#[test]
fn state_is_available_via_extractor() {
    let app = RustApi::new().state(123u32);
    let router = app.into_router();

    let req = http::Request::builder()
        .method(Method::GET)
        .uri("/test")
        .body(())
        .unwrap();
    let (parts, _) = req.into_parts();

    let request = Request::new(
        parts,
        crate::request::BodyVariant::Buffered(Bytes::new()),
        router.state_ref(),
        PathParams::new(),
    );
    let State(value) = State::<u32>::from_request_parts(&request).unwrap();
    assert_eq!(value, 123u32);
}

#[test]
fn test_path_param_type_inference_integer() {
    use super::helpers::infer_path_param_schema;

    // Test common integer patterns
    let int_params = [
        "page",
        "limit",
        "offset",
        "count",
        "item_count",
        "year",
        "month",
        "day",
        "index",
        "position",
    ];

    for name in int_params {
        let schema = infer_path_param_schema(name);
        match schema {
            rustapi_openapi::SchemaRef::Inline(v) => {
                assert_eq!(
                    v.get("type").and_then(|v| v.as_str()),
                    Some("integer"),
                    "Expected '{}' to be inferred as integer",
                    name
                );
            }
            _ => panic!("Expected inline schema for '{}'", name),
        }
    }
}

#[test]
fn test_path_param_type_inference_uuid() {
    use super::helpers::infer_path_param_schema;

    // Test UUID patterns
    let uuid_params = ["uuid", "user_uuid", "sessionUuid"];

    for name in uuid_params {
        let schema = infer_path_param_schema(name);
        match schema {
            rustapi_openapi::SchemaRef::Inline(v) => {
                assert_eq!(
                    v.get("type").and_then(|v| v.as_str()),
                    Some("string"),
                    "Expected '{}' to be inferred as string",
                    name
                );
                assert_eq!(
                    v.get("format").and_then(|v| v.as_str()),
                    Some("uuid"),
                    "Expected '{}' to have uuid format",
                    name
                );
            }
            _ => panic!("Expected inline schema for '{}'", name),
        }
    }
}

#[test]
fn test_path_param_type_inference_string() {
    use super::helpers::infer_path_param_schema;

    // Test string (default) patterns
    let string_params = [
        "name", "slug", "code", "token", "username", "id", "user_id", "userId", "postId",
    ];

    for name in string_params {
        let schema = infer_path_param_schema(name);
        match schema {
            rustapi_openapi::SchemaRef::Inline(v) => {
                assert_eq!(
                    v.get("type").and_then(|v| v.as_str()),
                    Some("string"),
                    "Expected '{}' to be inferred as string",
                    name
                );
                assert!(
                    v.get("format").is_none()
                        || v.get("format").and_then(|v| v.as_str()) != Some("uuid"),
                    "Expected '{}' to NOT have uuid format",
                    name
                );
            }
            _ => panic!("Expected inline schema for '{}'", name),
        }
    }
}

#[test]
fn test_schema_type_to_openapi_schema() {
    use super::helpers::schema_type_to_openapi_schema;

    // Test UUID schema
    let uuid_schema = schema_type_to_openapi_schema("uuid");
    match uuid_schema {
        rustapi_openapi::SchemaRef::Inline(v) => {
            assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("string"));
            assert_eq!(v.get("format").and_then(|v| v.as_str()), Some("uuid"));
        }
        _ => panic!("Expected inline schema for uuid"),
    }

    // Test integer schemas
    for schema_type in ["integer", "int", "int64", "i64"] {
        let schema = schema_type_to_openapi_schema(schema_type);
        match schema {
            rustapi_openapi::SchemaRef::Inline(v) => {
                assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("integer"));
                assert_eq!(v.get("format").and_then(|v| v.as_str()), Some("int64"));
            }
            _ => panic!("Expected inline schema for {}", schema_type),
        }
    }

    // Test int32 schema
    let int32_schema = schema_type_to_openapi_schema("int32");
    match int32_schema {
        rustapi_openapi::SchemaRef::Inline(v) => {
            assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("integer"));
            assert_eq!(v.get("format").and_then(|v| v.as_str()), Some("int32"));
        }
        _ => panic!("Expected inline schema for int32"),
    }

    // Test number/float schema
    for schema_type in ["number", "float"] {
        let schema = schema_type_to_openapi_schema(schema_type);
        match schema {
            rustapi_openapi::SchemaRef::Inline(v) => {
                assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("number"));
            }
            _ => panic!("Expected inline schema for {}", schema_type),
        }
    }

    // Test boolean schema
    for schema_type in ["boolean", "bool"] {
        let schema = schema_type_to_openapi_schema(schema_type);
        match schema {
            rustapi_openapi::SchemaRef::Inline(v) => {
                assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("boolean"));
            }
            _ => panic!("Expected inline schema for {}", schema_type),
        }
    }

    // Test string schema (default)
    let string_schema = schema_type_to_openapi_schema("string");
    match string_schema {
        rustapi_openapi::SchemaRef::Inline(v) => {
            assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("string"));
        }
        _ => panic!("Expected inline schema for string"),
    }
}

// **Feature: router-nesting, Property 11: OpenAPI Integration**
//
// For any nested routes with OpenAPI operations, the operations should appear
// in the parent's OpenAPI spec with prefixed paths and preserved metadata.
//
// **Validates: Requirements 4.1, 4.2**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Nested routes appear in OpenAPI spec with prefixed paths
    ///
    /// For any router with routes nested under a prefix, all routes should
    /// appear in the OpenAPI spec with the prefix prepended to their paths.
    #[test]
    fn prop_nested_routes_in_openapi_spec(
        // Generate prefix segments
        prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
        // Generate route path segments
        route_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
        has_param in any::<bool>(),
    ) {
        async fn handler() -> &'static str { "handler" }

        // Build the prefix
        let prefix = format!("/{}", prefix_segments.join("/"));

        // Build the route path
        let mut route_path = format!("/{}", route_segments.join("/"));
        if has_param {
            route_path.push_str("/{id}");
        }

        // Create nested router and nest it through RustApi
        let nested_router = Router::new().route(&route_path, get(handler));
        let app = RustApi::new().nest(&prefix, nested_router);

        // Build expected prefixed path for OpenAPI (uses {param} format)
        let expected_openapi_path = format!("{}{}", prefix, route_path);

        // Get the OpenAPI spec
        let spec = app.openapi_spec();

        // Property: The prefixed route should exist in OpenAPI paths
        prop_assert!(
            spec.paths.contains_key(&expected_openapi_path),
            "Expected OpenAPI path '{}' not found. Available paths: {:?}",
            expected_openapi_path,
            spec.paths.keys().collect::<Vec<_>>()
        );

        // Property: The path item should have a GET operation
        let path_item = spec.paths.get(&expected_openapi_path).unwrap();
        prop_assert!(
            path_item.get.is_some(),
            "GET operation should exist for path '{}'",
            expected_openapi_path
        );
    }

    /// Property: Multiple HTTP methods are preserved in OpenAPI spec after nesting
    ///
    /// For any router with routes having multiple HTTP methods, nesting should
    /// preserve all method operations in the OpenAPI spec.
    #[test]
    fn prop_multiple_methods_preserved_in_openapi(
        prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
        route_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
    ) {
        async fn get_handler() -> &'static str { "get" }
        async fn post_handler() -> &'static str { "post" }

        // Build the prefix and route path
        let prefix = format!("/{}", prefix_segments.join("/"));
        let route_path = format!("/{}", route_segments.join("/"));

        // Create nested router with both GET and POST using separate routes
        // Since MethodRouter doesn't have chaining methods, we create two routes
        let get_route_path = format!("{}/get", route_path);
        let post_route_path = format!("{}/post", route_path);
        let nested_router = Router::new()
            .route(&get_route_path, get(get_handler))
            .route(&post_route_path, post(post_handler));
        let app = RustApi::new().nest(&prefix, nested_router);

        // Build expected prefixed paths for OpenAPI
        let expected_get_path = format!("{}{}", prefix, get_route_path);
        let expected_post_path = format!("{}{}", prefix, post_route_path);

        // Get the OpenAPI spec
        let spec = app.openapi_spec();

        // Property: Both paths should exist
        prop_assert!(
            spec.paths.contains_key(&expected_get_path),
            "Expected OpenAPI path '{}' not found",
            expected_get_path
        );
        prop_assert!(
            spec.paths.contains_key(&expected_post_path),
            "Expected OpenAPI path '{}' not found",
            expected_post_path
        );

        // Property: GET operation should exist on get path
        let get_path_item = spec.paths.get(&expected_get_path).unwrap();
        prop_assert!(
            get_path_item.get.is_some(),
            "GET operation should exist for path '{}'",
            expected_get_path
        );

        // Property: POST operation should exist on post path
        let post_path_item = spec.paths.get(&expected_post_path).unwrap();
        prop_assert!(
            post_path_item.post.is_some(),
            "POST operation should exist for path '{}'",
            expected_post_path
        );
    }

    /// Property: Path parameters are added to OpenAPI operations after nesting
    ///
    /// For any nested route with path parameters, the OpenAPI operation should
    /// include the path parameters.
    #[test]
    fn prop_path_params_in_openapi_after_nesting(
        prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
        param_name in "[a-z][a-z0-9]{0,5}",
    ) {
        async fn handler() -> &'static str { "handler" }

        // Build the prefix and route path with parameter
        let prefix = format!("/{}", prefix_segments.join("/"));
        let route_path = format!("/{{{}}}", param_name);

        // Create nested router
        let nested_router = Router::new().route(&route_path, get(handler));
        let app = RustApi::new().nest(&prefix, nested_router);

        // Build expected prefixed path for OpenAPI
        let expected_openapi_path = format!("{}{}", prefix, route_path);

        // Get the OpenAPI spec
        let spec = app.openapi_spec();

        // Property: The path should exist
        prop_assert!(
            spec.paths.contains_key(&expected_openapi_path),
            "Expected OpenAPI path '{}' not found",
            expected_openapi_path
        );

        // Property: The GET operation should have the path parameter
        let path_item = spec.paths.get(&expected_openapi_path).unwrap();
        let get_op = path_item.get.as_ref().unwrap();

        prop_assert!(
            !get_op.parameters.is_empty(),
            "Operation should have parameters for path '{}'",
            expected_openapi_path
        );

        let params = &get_op.parameters;
        let has_param = params.iter().any(|p| p.name == param_name && p.location == "path");
        prop_assert!(
            has_param,
            "Path parameter '{}' should exist in operation parameters. Found: {:?}",
            param_name,
            params.iter().map(|p| &p.name).collect::<Vec<_>>()
        );
    }
}

// **Feature: router-nesting, Property 13: RustApi Integration**
//
// For any router nested through `RustApi::new().nest()`, the behavior should be
// identical to nesting through `Router::new().nest()`, and routes should appear
// in the OpenAPI spec.
//
// **Validates: Requirements 6.1, 6.2**
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: RustApi::nest delegates to Router::nest and produces identical route registration
    ///
    /// For any router with routes nested under a prefix, nesting through RustApi
    /// should produce the same route registration as nesting through Router directly.
    #[test]
    fn prop_rustapi_nest_delegates_to_router_nest(
        prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
        route_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
        has_param in any::<bool>(),
    ) {
        async fn handler() -> &'static str { "handler" }

        // Build the prefix
        let prefix = format!("/{}", prefix_segments.join("/"));

        // Build the route path
        let mut route_path = format!("/{}", route_segments.join("/"));
        if has_param {
            route_path.push_str("/{id}");
        }

        // Create nested router
        let nested_router_for_rustapi = Router::new().route(&route_path, get(handler));
        let nested_router_for_router = Router::new().route(&route_path, get(handler));

        // Nest through RustApi
        let rustapi_app = RustApi::new().nest(&prefix, nested_router_for_rustapi);
        let rustapi_router = rustapi_app.into_router();

        // Nest through Router directly
        let router_app = Router::new().nest(&prefix, nested_router_for_router);

        // Property: Both should have the same registered routes
        let rustapi_routes = rustapi_router.registered_routes();
        let router_routes = router_app.registered_routes();

        prop_assert_eq!(
            rustapi_routes.len(),
            router_routes.len(),
            "RustApi and Router should have same number of routes"
        );

        // Property: All routes from Router should exist in RustApi
        for (path, info) in router_routes {
            prop_assert!(
                rustapi_routes.contains_key(path),
                "Route '{}' from Router should exist in RustApi routes",
                path
            );

            let rustapi_info = rustapi_routes.get(path).unwrap();
            prop_assert_eq!(
                &info.path, &rustapi_info.path,
                "Display paths should match for route '{}'",
                path
            );
            prop_assert_eq!(
                info.methods.len(), rustapi_info.methods.len(),
                "Method count should match for route '{}'",
                path
            );
        }
    }

    /// Property: RustApi::nest includes nested routes in OpenAPI spec
    ///
    /// For any router with routes nested through RustApi, all routes should
    /// appear in the OpenAPI specification with prefixed paths.
    #[test]
    fn prop_rustapi_nest_includes_routes_in_openapi(
        prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
        route_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
        has_param in any::<bool>(),
    ) {
        async fn handler() -> &'static str { "handler" }

        // Build the prefix
        let prefix = format!("/{}", prefix_segments.join("/"));

        // Build the route path
        let mut route_path = format!("/{}", route_segments.join("/"));
        if has_param {
            route_path.push_str("/{id}");
        }

        // Create nested router and nest through RustApi
        let nested_router = Router::new().route(&route_path, get(handler));
        let app = RustApi::new().nest(&prefix, nested_router);

        // Build expected prefixed path for OpenAPI
        let expected_openapi_path = format!("{}{}", prefix, route_path);

        // Get the OpenAPI spec
        let spec = app.openapi_spec();

        // Property: The prefixed route should exist in OpenAPI paths
        prop_assert!(
            spec.paths.contains_key(&expected_openapi_path),
            "Expected OpenAPI path '{}' not found. Available paths: {:?}",
            expected_openapi_path,
            spec.paths.keys().collect::<Vec<_>>()
        );

        // Property: The path item should have a GET operation
        let path_item = spec.paths.get(&expected_openapi_path).unwrap();
        prop_assert!(
            path_item.get.is_some(),
            "GET operation should exist for path '{}'",
            expected_openapi_path
        );
    }

    /// Property: RustApi::nest route matching is identical to Router::nest
    ///
    /// For any nested route, matching through RustApi should produce the same
    /// result as matching through Router directly.
    #[test]
    fn prop_rustapi_nest_route_matching_identical(
        prefix_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..2),
        route_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..2),
        param_value in "[a-z0-9]{1,10}",
    ) {
        use crate::router::RouteMatch;

        async fn handler() -> &'static str { "handler" }

        // Build the prefix and route path with parameter
        let prefix = format!("/{}", prefix_segments.join("/"));
        let route_path = format!("/{}/{{id}}", route_segments.join("/"));

        // Create nested routers
        let nested_router_for_rustapi = Router::new().route(&route_path, get(handler));
        let nested_router_for_router = Router::new().route(&route_path, get(handler));

        // Nest through both RustApi and Router
        let rustapi_app = RustApi::new().nest(&prefix, nested_router_for_rustapi);
        let rustapi_router = rustapi_app.into_router();
        let router_app = Router::new().nest(&prefix, nested_router_for_router);

        // Build the full path to match
        let full_path = format!("{}/{}/{}", prefix, route_segments.join("/"), param_value);

        // Match through both
        let rustapi_match = rustapi_router.match_route(&full_path, &Method::GET);
        let router_match = router_app.match_route(&full_path, &Method::GET);

        // Property: Both should return Found with same parameters
        match (rustapi_match, router_match) {
            (RouteMatch::Found { params: rustapi_params, .. }, RouteMatch::Found { params: router_params, .. }) => {
                prop_assert_eq!(
                    rustapi_params.len(),
                    router_params.len(),
                    "Parameter count should match"
                );
                for (key, value) in &router_params {
                    prop_assert!(
                        rustapi_params.contains_key(key),
                        "RustApi should have parameter '{}'",
                        key
                    );
                    prop_assert_eq!(
                        rustapi_params.get(key).unwrap(),
                        value,
                        "Parameter '{}' value should match",
                        key
                    );
                }
            }
            (rustapi_result, router_result) => {
                prop_assert!(
                    false,
                    "Both should return Found, but RustApi returned {:?} and Router returned {:?}",
                    match rustapi_result {
                        RouteMatch::Found { .. } => "Found",
                        RouteMatch::NotFound => "NotFound",
                        RouteMatch::MethodNotAllowed { .. } => "MethodNotAllowed",
                    },
                    match router_result {
                        RouteMatch::Found { .. } => "Found",
                        RouteMatch::NotFound => "NotFound",
                        RouteMatch::MethodNotAllowed { .. } => "MethodNotAllowed",
                    }
                );
            }
        }
    }
}

/// Unit test: Verify OpenAPI operations are propagated during nesting
#[test]
fn test_openapi_operations_propagated_during_nesting() {
    async fn list_users() -> &'static str {
        "list users"
    }
    async fn get_user() -> &'static str {
        "get user"
    }
    async fn create_user() -> &'static str {
        "create user"
    }

    // Create nested router with multiple routes
    // Note: We use separate routes since MethodRouter doesn't support chaining
    let users_router = Router::new()
        .route("/", get(list_users))
        .route("/create", post(create_user))
        .route("/{id}", get(get_user));

    // Nest under /api/v1/users
    let app = RustApi::new().nest("/api/v1/users", users_router);

    let spec = app.openapi_spec();

    // Verify /api/v1/users path exists with GET
    assert!(
        spec.paths.contains_key("/api/v1/users"),
        "Should have /api/v1/users path"
    );
    let users_path = spec.paths.get("/api/v1/users").unwrap();
    assert!(users_path.get.is_some(), "Should have GET operation");

    // Verify /api/v1/users/create path exists with POST
    assert!(
        spec.paths.contains_key("/api/v1/users/create"),
        "Should have /api/v1/users/create path"
    );
    let create_path = spec.paths.get("/api/v1/users/create").unwrap();
    assert!(create_path.post.is_some(), "Should have POST operation");

    // Verify /api/v1/users/{id} path exists with GET
    assert!(
        spec.paths.contains_key("/api/v1/users/{id}"),
        "Should have /api/v1/users/{{id}} path"
    );
    let user_path = spec.paths.get("/api/v1/users/{id}").unwrap();
    assert!(
        user_path.get.is_some(),
        "Should have GET operation for user by id"
    );

    // Verify path parameter is added
    let get_user_op = user_path.get.as_ref().unwrap();
    assert!(!get_user_op.parameters.is_empty(), "Should have parameters");
    let params = &get_user_op.parameters;
    assert!(
        params
            .iter()
            .any(|p| p.name == "id" && p.location == "path"),
        "Should have 'id' path parameter"
    );
}

/// Unit test: Verify nested routes don't appear without nesting
#[test]
fn test_openapi_spec_empty_without_routes() {
    let app = RustApi::new();
    let spec = app.openapi_spec();

    // Should have no paths (except potentially default ones)
    assert!(
        spec.paths.is_empty(),
        "OpenAPI spec should have no paths without routes"
    );
}

/// Unit test: Verify RustApi::nest delegates correctly to Router::nest
///
/// **Feature: router-nesting, Property 13: RustApi Integration**
/// **Validates: Requirements 6.1, 6.2**
#[test]
fn test_rustapi_nest_delegates_to_router_nest() {
    use crate::router::RouteMatch;

    async fn list_users() -> &'static str {
        "list users"
    }
    async fn get_user() -> &'static str {
        "get user"
    }
    async fn create_user() -> &'static str {
        "create user"
    }

    // Create nested router with multiple routes
    let users_router = Router::new()
        .route("/", get(list_users))
        .route("/create", post(create_user))
        .route("/{id}", get(get_user));

    // Nest through RustApi
    let app = RustApi::new().nest("/api/v1/users", users_router);
    let router = app.into_router();

    // Verify routes are registered correctly
    let routes = router.registered_routes();
    assert_eq!(routes.len(), 3, "Should have 3 routes registered");

    // Verify route paths
    assert!(
        routes.contains_key("/api/v1/users"),
        "Should have /api/v1/users route"
    );
    assert!(
        routes.contains_key("/api/v1/users/create"),
        "Should have /api/v1/users/create route"
    );
    assert!(
        routes.contains_key("/api/v1/users/:id"),
        "Should have /api/v1/users/:id route"
    );

    // Verify route matching works
    match router.match_route("/api/v1/users", &Method::GET) {
        RouteMatch::Found { params, .. } => {
            assert!(params.is_empty(), "Root route should have no params");
        }
        _ => panic!("GET /api/v1/users should be found"),
    }

    match router.match_route("/api/v1/users/create", &Method::POST) {
        RouteMatch::Found { params, .. } => {
            assert!(params.is_empty(), "Create route should have no params");
        }
        _ => panic!("POST /api/v1/users/create should be found"),
    }

    match router.match_route("/api/v1/users/123", &Method::GET) {
        RouteMatch::Found { params, .. } => {
            assert_eq!(
                params.get("id"),
                Some(&"123".to_string()),
                "Should extract id param"
            );
        }
        _ => panic!("GET /api/v1/users/123 should be found"),
    }

    // Verify method not allowed
    match router.match_route("/api/v1/users", &Method::DELETE) {
        RouteMatch::MethodNotAllowed { allowed } => {
            assert!(allowed.contains(&Method::GET), "Should allow GET");
        }
        _ => panic!("DELETE /api/v1/users should return MethodNotAllowed"),
    }
}

/// Unit test: Verify RustApi::nest includes routes in OpenAPI spec
///
/// **Feature: router-nesting, Property 13: RustApi Integration**
/// **Validates: Requirements 6.1, 6.2**
#[test]
fn test_rustapi_nest_includes_routes_in_openapi_spec() {
    async fn list_items() -> &'static str {
        "list items"
    }
    async fn get_item() -> &'static str {
        "get item"
    }

    // Create nested router
    let items_router = Router::new()
        .route("/", get(list_items))
        .route("/{item_id}", get(get_item));

    // Nest through RustApi
    let app = RustApi::new().nest("/api/items", items_router);

    // Verify OpenAPI spec
    let spec = app.openapi_spec();

    // Verify paths exist
    assert!(
        spec.paths.contains_key("/api/items"),
        "Should have /api/items in OpenAPI"
    );
    assert!(
        spec.paths.contains_key("/api/items/{item_id}"),
        "Should have /api/items/{{item_id}} in OpenAPI"
    );

    // Verify operations
    let list_path = spec.paths.get("/api/items").unwrap();
    assert!(
        list_path.get.is_some(),
        "Should have GET operation for /api/items"
    );

    let get_path = spec.paths.get("/api/items/{item_id}").unwrap();
    assert!(
        get_path.get.is_some(),
        "Should have GET operation for /api/items/{{item_id}}"
    );

    // Verify path parameter is added
    let get_op = get_path.get.as_ref().unwrap();
    assert!(!get_op.parameters.is_empty(), "Should have parameters");
    let params = &get_op.parameters;
    assert!(
        params
            .iter()
            .any(|p| p.name == "item_id" && p.location == "path"),
        "Should have 'item_id' path parameter"
    );
}

use crate::health::{HealthCheckBuilder, HealthEndpointConfig, HealthStatus};
use crate::ProductionDefaultsConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

fn bind_ephemeral_port() -> (u16, String) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    (port, format!("127.0.0.1:{port}"))
}

#[tokio::test]
async fn run_with_shutdown_exposes_default_health_endpoints() {
    let app = RustApi::new().health_endpoints();
    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    for path in ["/health", "/ready", "/live"] {
        let res = client
            .get(format!("{base_url}{path}"))
            .send()
            .await
            .expect("health endpoint request failed");
        assert_eq!(res.status(), 200, "{path} should return 200");
        let body: serde_json::Value = res.json().await.unwrap();
        assert!(body.get("status").is_some(), "{path} should include status");
        assert!(
            body.get("timestamp").is_some(),
            "{path} should include timestamp"
        );
    }

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn run_with_shutdown_returns_503_for_unhealthy_readiness() {
    let health = HealthCheckBuilder::new(false)
        .add_check("database", || async {
            HealthStatus::unhealthy("database offline")
        })
        .build();
    let app = RustApi::new().with_health_check(health);
    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let res = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/ready"))
        .send()
        .await
        .expect("readiness endpoint request failed");
    assert_eq!(res.status(), 503);
    assert!(res.text().await.unwrap().contains("database offline"));

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn run_with_shutdown_honors_custom_health_paths() {
    let config = HealthEndpointConfig::new()
        .health_path("/healthz")
        .readiness_path("/readyz")
        .liveness_path("/livez");
    let app = RustApi::new().health_endpoints_with_config(config);
    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    for path in ["/healthz", "/readyz", "/livez"] {
        let res = client
            .get(format!("{base_url}{path}"))
            .send()
            .await
            .expect("custom health path request failed");
        assert_eq!(res.status(), 200, "{path} should return 200");
    }

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn run_with_shutdown_serves_status_page_and_tracks_requests() {
    async fn task_handler() -> &'static str {
        "ok"
    }

    let app = RustApi::new()
        .status_page()
        .route("/task", get(task_handler));
    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    let res = client
        .get(format!("{base_url}/status"))
        .send()
        .await
        .expect("status page request failed");
    assert_eq!(res.status(), 200);
    let body = res.text().await.unwrap();
    assert!(body.contains("System Status"));
    assert!(body.contains("Total Requests"));

    for _ in 0..2 {
        let res = client
            .get(format!("{base_url}/task"))
            .send()
            .await
            .expect("task request failed");
        assert_eq!(res.status(), 200);
    }

    let res = client
        .get(format!("{base_url}/status"))
        .send()
        .await
        .expect("updated status page request failed");
    let body = res.text().await.unwrap();
    assert!(body.contains("Total Requests"));

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn run_with_shutdown_runs_lifecycle_hooks() {
    let on_start = Arc::new(AtomicBool::new(false));
    let on_shutdown = Arc::new(AtomicBool::new(false));
    let on_start_flag = on_start.clone();
    let on_shutdown_flag = on_shutdown.clone();

    let app = RustApi::new()
        .health_endpoints()
        .on_start(move || {
            let on_start_flag = on_start_flag.clone();
            async move {
                on_start_flag.store(true, Ordering::SeqCst);
            }
        })
        .on_shutdown(move || {
            let on_shutdown_flag = on_shutdown_flag.clone();
            async move {
                on_shutdown_flag.store(true, Ordering::SeqCst);
            }
        });

    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        on_start.load(Ordering::SeqCst),
        "on_start should run via prepare_for_serve before accept loop"
    );

    let res = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/health"))
        .send()
        .await
        .expect("health probe failed");
    assert_eq!(res.status(), 200);

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
    assert!(
        on_shutdown.load(Ordering::SeqCst),
        "on_shutdown should run after shutdown signal"
    );
}

#[tokio::test]
async fn run_with_shutdown_applies_production_defaults() {
    async fn hello() -> &'static str {
        "ok"
    }

    let app = RustApi::new()
        .production_defaults("users-api")
        .route("/hello", get(hello));
    assert_eq!(app.layers().len(), 2);

    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    let res = client
        .get(format!("{base_url}/hello"))
        .send()
        .await
        .expect("hello request failed");
    assert_eq!(res.status(), 200);
    assert!(res.headers().get("x-request-id").is_some());

    let res = client
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("health request failed");
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body.get("version").is_none());
    assert!(body.get("checks").is_some());

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn run_with_shutdown_applies_custom_production_defaults() {
    let config = ProductionDefaultsConfig::new("billing-api")
        .version("1.2.3")
        .health_endpoint_config(
            HealthEndpointConfig::new()
                .health_path("/healthz")
                .readiness_path("/readyz")
                .liveness_path("/livez"),
        );

    let app = RustApi::new().production_defaults_with_config(config);
    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    for path in ["/healthz", "/readyz", "/livez"] {
        let res = client
            .get(format!("{base_url}{path}"))
            .send()
            .await
            .expect("probe request failed");
        assert_eq!(res.status(), 200);
    }

    let res = client
        .get(format!("{base_url}/healthz"))
        .send()
        .await
        .expect("healthz request failed");
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(
        body.get("version"),
        Some(&serde_json::Value::String("1.2.3".to_string()))
    );

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn run_with_shutdown_serves_registered_routes() {
    async fn ping() -> &'static str {
        "pong"
    }

    let app = RustApi::new().route("/ping", get(ping));
    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let res = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/ping"))
        .send()
        .await
        .expect("ping request failed");
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "pong");

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn run_with_shutdown_enables_hot_reload_pipeline() {
    struct HotReloadEnvGuard(Option<String>);

    impl HotReloadEnvGuard {
        fn clear() -> Self {
            let previous = std::env::var("RUSTAPI_HOT_RELOAD").ok();
            std::env::remove_var("RUSTAPI_HOT_RELOAD");
            Self(previous)
        }
    }

    impl Drop for HotReloadEnvGuard {
        fn drop(&mut self) {
            match &self.0 {
                Some(value) => std::env::set_var("RUSTAPI_HOT_RELOAD", value),
                None => std::env::remove_var("RUSTAPI_HOT_RELOAD"),
            }
        }
    }

    let _guard = HotReloadEnvGuard::clear();
    async fn ok_handler() -> &'static str {
        "ok"
    }
    let app = RustApi::new()
        .hot_reload(true)
        .route("/ping", get(ok_handler));
    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        std::env::var("RUSTAPI_HOT_RELOAD").ok().as_deref(),
        Some("1"),
        "prepare_for_serve should set hot-reload env via run entrypoint"
    );

    let res = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/ping"))
        .send()
        .await
        .expect("ping request failed");
    assert_eq!(res.status(), 200);

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn run_with_shutdown_composes_health_and_status_endpoints() {
    let app = RustApi::new().health_endpoints().status_page();
    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    for path in ["/health", "/status"] {
        let res = client
            .get(format!("{base_url}{path}"))
            .send()
            .await
            .unwrap_or_else(|_| panic!("request to {path} failed"));
        assert_eq!(res.status(), 200, "{path} should be reachable");
    }

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn run_with_shutdown_serves_nested_routes() {
    async fn nested() -> &'static str {
        "nested-ok"
    }

    let app = RustApi::new().nest("/api", Router::new().route("/items", get(nested)));
    let (port, addr) = bind_ephemeral_port();
    let (tx, rx) = oneshot::channel();

    let server = tokio::spawn(async move {
        app.run_with_shutdown(&addr, async {
            rx.await.ok();
        })
        .await
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    let res = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/api/items"))
        .send()
        .await
        .expect("nested route request failed");
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "nested-ok");

    tx.send(()).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[test]
fn production_defaults_can_disable_optional_layers() {
    let app = RustApi::new().production_defaults_with_config(
        ProductionDefaultsConfig::new("minimal-api")
            .request_id(false)
            .tracing(false)
            .health_endpoints(false),
    );

    assert_eq!(app.layers().len(), 0);
}
