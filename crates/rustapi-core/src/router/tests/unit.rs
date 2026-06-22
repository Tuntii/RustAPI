use crate::router::{
    convert_path_params, get, normalize_path_for_comparison, normalize_prefix, post, put,
    MethodRouter, RouteMatch, Router,
};
use http::Method;

#[test]
fn test_convert_path_params() {
    assert_eq!(convert_path_params("/users/{id}"), "/users/:id");
    assert_eq!(
        convert_path_params("/users/{user_id}/posts/{post_id}"),
        "/users/:user_id/posts/:post_id"
    );
    assert_eq!(convert_path_params("/static/path"), "/static/path");
}

#[test]
fn test_normalize_path_for_comparison() {
    assert_eq!(normalize_path_for_comparison("/users/:id"), "/users/:_");
    assert_eq!(
        normalize_path_for_comparison("/users/:user_id"),
        "/users/:_"
    );
    assert_eq!(
        normalize_path_for_comparison("/users/:id/posts/:post_id"),
        "/users/:_/posts/:_"
    );
    assert_eq!(
        normalize_path_for_comparison("/static/path"),
        "/static/path"
    );
}

#[test]
fn test_normalize_prefix() {
    // Basic cases
    assert_eq!(normalize_prefix("api"), "/api");
    assert_eq!(normalize_prefix("/api"), "/api");
    assert_eq!(normalize_prefix("/api/"), "/api");
    assert_eq!(normalize_prefix("api/"), "/api");

    // Multiple segments
    assert_eq!(normalize_prefix("api/v1"), "/api/v1");
    assert_eq!(normalize_prefix("/api/v1"), "/api/v1");
    assert_eq!(normalize_prefix("/api/v1/"), "/api/v1");

    // Edge cases: empty and root
    assert_eq!(normalize_prefix(""), "/");
    assert_eq!(normalize_prefix("/"), "/");

    // Multiple slashes
    assert_eq!(normalize_prefix("//api"), "/api");
    assert_eq!(normalize_prefix("api//v1"), "/api/v1");
    assert_eq!(normalize_prefix("//api//v1//"), "/api/v1");
    assert_eq!(normalize_prefix("///"), "/");
}

#[test]
#[should_panic(expected = "ROUTE CONFLICT DETECTED")]
fn test_route_conflict_detection() {
    async fn handler1() -> &'static str {
        "handler1"
    }
    async fn handler2() -> &'static str {
        "handler2"
    }

    let _router = Router::new()
        .route("/users/{id}", get(handler1))
        .route("/users/{user_id}", get(handler2)); // This should panic
}

#[test]
fn test_no_conflict_different_paths() {
    async fn handler1() -> &'static str {
        "handler1"
    }
    async fn handler2() -> &'static str {
        "handler2"
    }

    let router = Router::new()
        .route("/users/{id}", get(handler1))
        .route("/users/{id}/profile", get(handler2));

    assert_eq!(router.registered_routes().len(), 2);
}

#[test]
fn test_route_info_tracking() {
    async fn handler() -> &'static str {
        "handler"
    }

    let router = Router::new().route("/users/{id}", get(handler));

    let routes = router.registered_routes();
    assert_eq!(routes.len(), 1);

    let info = routes.get("/users/:id").unwrap();
    assert_eq!(info.path, "/users/{id}");
    assert_eq!(info.methods.len(), 1);
    assert_eq!(info.methods[0], Method::GET);
}

#[test]
fn test_basic_router_nesting() {
    async fn list_users() -> &'static str {
        "list users"
    }
    async fn get_user() -> &'static str {
        "get user"
    }

    let users_router = Router::new()
        .route("/", get(list_users))
        .route("/{id}", get(get_user));

    let app = Router::new().nest("/api/users", users_router);

    let routes = app.registered_routes();
    assert_eq!(routes.len(), 2);

    // Check that routes are registered with prefix
    assert!(routes.contains_key("/api/users"));
    assert!(routes.contains_key("/api/users/:id"));

    // Check display paths
    let list_info = routes.get("/api/users").unwrap();
    assert_eq!(list_info.path, "/api/users");

    let get_info = routes.get("/api/users/:id").unwrap();
    assert_eq!(get_info.path, "/api/users/{id}");
}

#[test]
fn test_nested_route_matching() {
    async fn handler() -> &'static str {
        "handler"
    }

    let users_router = Router::new().route("/{id}", get(handler));

    let app = Router::new().nest("/api/users", users_router);

    // Test that the route can be matched
    match app.match_route("/api/users/123", &Method::GET) {
        RouteMatch::Found { params, .. } => {
            assert_eq!(params.get("id"), Some(&"123".to_string()));
        }
        _ => panic!("Route should be found"),
    }
}

#[test]
fn test_nested_route_matching_multiple_params() {
    async fn handler() -> &'static str {
        "handler"
    }

    let posts_router = Router::new().route("/{user_id}/posts/{post_id}", get(handler));

    let app = Router::new().nest("/api", posts_router);

    // Test that multiple parameters are correctly extracted
    match app.match_route("/api/42/posts/100", &Method::GET) {
        RouteMatch::Found { params, .. } => {
            assert_eq!(params.get("user_id"), Some(&"42".to_string()));
            assert_eq!(params.get("post_id"), Some(&"100".to_string()));
        }
        _ => panic!("Route should be found"),
    }
}

#[test]
fn test_nested_route_matching_static_path() {
    async fn handler() -> &'static str {
        "handler"
    }

    let health_router = Router::new().route("/health", get(handler));

    let app = Router::new().nest("/api/v1", health_router);

    // Test that static paths are correctly matched
    match app.match_route("/api/v1/health", &Method::GET) {
        RouteMatch::Found { params, .. } => {
            assert!(params.is_empty(), "Static path should have no params");
        }
        _ => panic!("Route should be found"),
    }
}

#[test]
fn test_nested_route_not_found() {
    async fn handler() -> &'static str {
        "handler"
    }

    let users_router = Router::new().route("/users", get(handler));

    let app = Router::new().nest("/api", users_router);

    // Test that non-existent paths return NotFound
    match app.match_route("/api/posts", &Method::GET) {
        RouteMatch::NotFound => {
            // Expected
        }
        _ => panic!("Route should not be found"),
    }

    // Test that wrong prefix returns NotFound
    match app.match_route("/v2/users", &Method::GET) {
        RouteMatch::NotFound => {
            // Expected
        }
        _ => panic!("Route with wrong prefix should not be found"),
    }
}

#[test]
fn test_nested_route_method_not_allowed() {
    async fn handler() -> &'static str {
        "handler"
    }

    let users_router = Router::new().route("/users", get(handler));

    let app = Router::new().nest("/api", users_router);

    // Test that wrong method returns MethodNotAllowed
    match app.match_route("/api/users", &Method::POST) {
        RouteMatch::MethodNotAllowed { allowed } => {
            assert!(allowed.contains(&Method::GET));
            assert!(!allowed.contains(&Method::POST));
        }
        _ => panic!("Should return MethodNotAllowed"),
    }
}

#[test]
fn test_nested_route_multiple_methods() {
    async fn get_handler() -> &'static str {
        "get"
    }
    async fn post_handler() -> &'static str {
        "post"
    }

    // Create a method router with both GET and POST
    let get_router = get(get_handler);
    let post_router = post(post_handler);
    let mut combined = MethodRouter::new();
    for (method, handler) in get_router.handlers {
        combined.handlers.insert(method, handler);
    }
    for (method, handler) in post_router.handlers {
        combined.handlers.insert(method, handler);
    }

    let users_router = Router::new().route("/users", combined);
    let app = Router::new().nest("/api", users_router);

    // Both GET and POST should work
    match app.match_route("/api/users", &Method::GET) {
        RouteMatch::Found { .. } => {}
        _ => panic!("GET should be found"),
    }

    match app.match_route("/api/users", &Method::POST) {
        RouteMatch::Found { .. } => {}
        _ => panic!("POST should be found"),
    }

    // DELETE should return MethodNotAllowed with GET and POST in allowed
    match app.match_route("/api/users", &Method::DELETE) {
        RouteMatch::MethodNotAllowed { allowed } => {
            assert!(allowed.contains(&Method::GET));
            assert!(allowed.contains(&Method::POST));
        }
        _ => panic!("DELETE should return MethodNotAllowed"),
    }
}

#[test]
fn test_nested_router_prefix_normalization() {
    async fn handler() -> &'static str {
        "handler"
    }

    // Test various prefix formats
    let router1 = Router::new().route("/test", get(handler));
    let app1 = Router::new().nest("api", router1);
    assert!(app1.registered_routes().contains_key("/api/test"));

    let router2 = Router::new().route("/test", get(handler));
    let app2 = Router::new().nest("/api/", router2);
    assert!(app2.registered_routes().contains_key("/api/test"));

    let router3 = Router::new().route("/test", get(handler));
    let app3 = Router::new().nest("//api//", router3);
    assert!(app3.registered_routes().contains_key("/api/test"));
}

#[test]
fn test_state_tracking() {
    #[derive(Clone)]
    struct MyState(#[allow(dead_code)] String);

    let router = Router::new().state(MyState("test".to_string()));

    assert!(router.has_state::<MyState>());
    assert!(!router.has_state::<String>());
}

#[test]
fn test_state_merge_nested_only() {
    #[derive(Clone, PartialEq, Debug)]
    struct NestedState(String);

    async fn handler() -> &'static str {
        "handler"
    }

    // Create a router with state to use as source for merging
    let state_source = Router::new().state(NestedState("nested".to_string()));

    let nested = Router::new().route("/test", get(handler));

    let parent = Router::new()
        .nest("/api", nested)
        .merge_state::<NestedState>(&state_source);

    // Parent should now have the nested state
    assert!(parent.has_state::<NestedState>());

    // Verify the state value
    let state = parent.state.get::<NestedState>().unwrap();
    assert_eq!(state.0, "nested");
}

#[test]
fn test_state_merge_parent_wins() {
    #[derive(Clone, PartialEq, Debug)]
    struct SharedState(String);

    async fn handler() -> &'static str {
        "handler"
    }

    // Create a router with state to use as source for merging
    let state_source = Router::new().state(SharedState("nested".to_string()));

    let nested = Router::new().route("/test", get(handler));

    let parent = Router::new()
        .state(SharedState("parent".to_string()))
        .nest("/api", nested)
        .merge_state::<SharedState>(&state_source);

    // Parent should still have its own state (parent wins)
    assert!(parent.has_state::<SharedState>());

    // Verify the state value is from parent
    let state = parent.state.get::<SharedState>().unwrap();
    assert_eq!(state.0, "parent");
}

#[test]
fn test_state_type_ids_merged_on_nest() {
    #[derive(Clone)]
    struct NestedState(#[allow(dead_code)] String);

    async fn handler() -> &'static str {
        "handler"
    }

    let nested = Router::new()
        .route("/test", get(handler))
        .state(NestedState("nested".to_string()));

    let parent = Router::new().nest("/api", nested);

    // Parent should track the nested state type ID
    assert!(parent
        .state_type_ids()
        .contains(&std::any::TypeId::of::<NestedState>()));
}

#[test]
#[should_panic(expected = "ROUTE CONFLICT DETECTED")]
fn test_nested_route_conflict_with_existing_route() {
    async fn handler1() -> &'static str {
        "handler1"
    }
    async fn handler2() -> &'static str {
        "handler2"
    }

    // Create a parent router with an existing route
    let parent = Router::new().route("/api/users/{id}", get(handler1));

    // Create a nested router with a conflicting route
    let nested = Router::new().route("/{user_id}", get(handler2));

    // This should panic because /api/users/{id} conflicts with /api/users/{user_id}
    let _app = parent.nest("/api/users", nested);
}

#[test]
#[should_panic(expected = "ROUTE CONFLICT DETECTED")]
fn test_nested_route_conflict_same_path_different_param_names() {
    async fn handler1() -> &'static str {
        "handler1"
    }
    async fn handler2() -> &'static str {
        "handler2"
    }

    // Create two nested routers with same path structure but different param names
    let nested1 = Router::new().route("/{id}", get(handler1));
    let nested2 = Router::new().route("/{user_id}", get(handler2));

    // Nest both under the same prefix - should conflict
    let _app = Router::new()
        .nest("/api/users", nested1)
        .nest("/api/users", nested2);
}

#[test]
fn test_nested_route_conflict_error_contains_both_paths() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    async fn handler1() -> &'static str {
        "handler1"
    }
    async fn handler2() -> &'static str {
        "handler2"
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        let parent = Router::new().route("/api/users/{id}", get(handler1));
        let nested = Router::new().route("/{user_id}", get(handler2));
        let _app = parent.nest("/api/users", nested);
    }));

    assert!(result.is_err(), "Should have panicked due to conflict");

    if let Err(panic_info) = result {
        if let Some(msg) = panic_info.downcast_ref::<String>() {
            assert!(
                msg.contains("ROUTE CONFLICT DETECTED"),
                "Error should contain 'ROUTE CONFLICT DETECTED'"
            );
            assert!(
                msg.contains("Existing:") && msg.contains("New:"),
                "Error should contain both 'Existing:' and 'New:' labels"
            );
            assert!(
                msg.contains("How to resolve:"),
                "Error should contain resolution guidance"
            );
        }
    }
}

#[test]
fn test_nested_routes_no_conflict_different_prefixes() {
    async fn handler1() -> &'static str {
        "handler1"
    }
    async fn handler2() -> &'static str {
        "handler2"
    }

    // Create two nested routers with same internal paths but different prefixes
    let nested1 = Router::new().route("/{id}", get(handler1));
    let nested2 = Router::new().route("/{id}", get(handler2));

    // Nest under different prefixes - should NOT conflict
    let app = Router::new()
        .nest("/api/users", nested1)
        .nest("/api/posts", nested2);

    assert_eq!(app.registered_routes().len(), 2);
    assert!(app.registered_routes().contains_key("/api/users/:id"));
    assert!(app.registered_routes().contains_key("/api/posts/:id"));
}

// **Feature: router-nesting, Property 4: Multiple Router Composition**
// Tests for nesting multiple routers under different prefixes
// **Validates: Requirements 1.5**

#[test]
fn test_multiple_router_composition_all_routes_registered() {
    async fn users_list() -> &'static str {
        "users list"
    }
    async fn users_get() -> &'static str {
        "users get"
    }
    async fn posts_list() -> &'static str {
        "posts list"
    }
    async fn posts_get() -> &'static str {
        "posts get"
    }
    async fn comments_list() -> &'static str {
        "comments list"
    }

    // Create multiple sub-routers with different routes
    let users_router = Router::new()
        .route("/", get(users_list))
        .route("/{id}", get(users_get));

    let posts_router = Router::new()
        .route("/", get(posts_list))
        .route("/{id}", get(posts_get));

    let comments_router = Router::new().route("/", get(comments_list));

    // Nest all routers under different prefixes
    let app = Router::new()
        .nest("/api/users", users_router)
        .nest("/api/posts", posts_router)
        .nest("/api/comments", comments_router);

    // Verify all routes are registered (2 + 2 + 1 = 5 routes)
    let routes = app.registered_routes();
    assert_eq!(routes.len(), 5, "Should have 5 routes registered");

    // Verify users routes
    assert!(
        routes.contains_key("/api/users"),
        "Should have /api/users route"
    );
    assert!(
        routes.contains_key("/api/users/:id"),
        "Should have /api/users/:id route"
    );

    // Verify posts routes
    assert!(
        routes.contains_key("/api/posts"),
        "Should have /api/posts route"
    );
    assert!(
        routes.contains_key("/api/posts/:id"),
        "Should have /api/posts/:id route"
    );

    // Verify comments routes
    assert!(
        routes.contains_key("/api/comments"),
        "Should have /api/comments route"
    );
}

#[test]
fn test_multiple_router_composition_no_interference() {
    async fn users_handler() -> &'static str {
        "users"
    }
    async fn posts_handler() -> &'static str {
        "posts"
    }
    async fn admin_handler() -> &'static str {
        "admin"
    }

    // Create routers with same internal structure but different prefixes
    let users_router = Router::new()
        .route("/list", get(users_handler))
        .route("/{id}", get(users_handler));

    let posts_router = Router::new()
        .route("/list", get(posts_handler))
        .route("/{id}", get(posts_handler));

    let admin_router = Router::new()
        .route("/list", get(admin_handler))
        .route("/{id}", get(admin_handler));

    // Nest all routers
    let app = Router::new()
        .nest("/api/v1/users", users_router)
        .nest("/api/v1/posts", posts_router)
        .nest("/admin", admin_router);

    // Verify all routes are registered (2 + 2 + 2 = 6 routes)
    let routes = app.registered_routes();
    assert_eq!(routes.len(), 6, "Should have 6 routes registered");

    // Verify each prefix group has its routes
    assert!(routes.contains_key("/api/v1/users/list"));
    assert!(routes.contains_key("/api/v1/users/:id"));
    assert!(routes.contains_key("/api/v1/posts/list"));
    assert!(routes.contains_key("/api/v1/posts/:id"));
    assert!(routes.contains_key("/admin/list"));
    assert!(routes.contains_key("/admin/:id"));

    // Verify routes are matchable and don't interfere with each other
    match app.match_route("/api/v1/users/list", &Method::GET) {
        RouteMatch::Found { params, .. } => {
            assert!(params.is_empty(), "Static path should have no params");
        }
        _ => panic!("Should find /api/v1/users/list"),
    }

    match app.match_route("/api/v1/posts/123", &Method::GET) {
        RouteMatch::Found { params, .. } => {
            assert_eq!(params.get("id"), Some(&"123".to_string()));
        }
        _ => panic!("Should find /api/v1/posts/123"),
    }

    match app.match_route("/admin/456", &Method::GET) {
        RouteMatch::Found { params, .. } => {
            assert_eq!(params.get("id"), Some(&"456".to_string()));
        }
        _ => panic!("Should find /admin/456"),
    }
}

#[test]
fn test_multiple_router_composition_with_multiple_methods() {
    async fn get_handler() -> &'static str {
        "get"
    }
    async fn post_handler() -> &'static str {
        "post"
    }
    async fn put_handler() -> &'static str {
        "put"
    }

    // Create routers with multiple HTTP methods
    // Combine GET and POST for users root
    let get_router = get(get_handler);
    let post_router = post(post_handler);
    let mut users_root_combined = MethodRouter::new();
    for (method, handler) in get_router.handlers {
        users_root_combined.handlers.insert(method, handler);
    }
    for (method, handler) in post_router.handlers {
        users_root_combined.handlers.insert(method, handler);
    }

    // Combine GET and PUT for users/{id}
    let get_router2 = get(get_handler);
    let put_router = put(put_handler);
    let mut users_id_combined = MethodRouter::new();
    for (method, handler) in get_router2.handlers {
        users_id_combined.handlers.insert(method, handler);
    }
    for (method, handler) in put_router.handlers {
        users_id_combined.handlers.insert(method, handler);
    }

    let users_router = Router::new()
        .route("/", users_root_combined)
        .route("/{id}", users_id_combined);

    // Combine GET and POST for posts root
    let get_router3 = get(get_handler);
    let post_router2 = post(post_handler);
    let mut posts_root_combined = MethodRouter::new();
    for (method, handler) in get_router3.handlers {
        posts_root_combined.handlers.insert(method, handler);
    }
    for (method, handler) in post_router2.handlers {
        posts_root_combined.handlers.insert(method, handler);
    }

    let posts_router = Router::new().route("/", posts_root_combined);

    // Nest routers
    let app = Router::new()
        .nest("/users", users_router)
        .nest("/posts", posts_router);

    // Verify routes are registered
    let routes = app.registered_routes();
    assert_eq!(routes.len(), 3, "Should have 3 routes registered");

    // Verify methods are preserved for users routes
    let users_root = routes.get("/users").unwrap();
    assert!(users_root.methods.contains(&Method::GET));
    assert!(users_root.methods.contains(&Method::POST));

    let users_id = routes.get("/users/:id").unwrap();
    assert!(users_id.methods.contains(&Method::GET));
    assert!(users_id.methods.contains(&Method::PUT));

    // Verify methods are preserved for posts routes
    let posts_root = routes.get("/posts").unwrap();
    assert!(posts_root.methods.contains(&Method::GET));
    assert!(posts_root.methods.contains(&Method::POST));

    // Verify route matching works for all methods
    match app.match_route("/users", &Method::GET) {
        RouteMatch::Found { .. } => {}
        _ => panic!("GET /users should be found"),
    }
    match app.match_route("/users", &Method::POST) {
        RouteMatch::Found { .. } => {}
        _ => panic!("POST /users should be found"),
    }
    match app.match_route("/users/123", &Method::PUT) {
        RouteMatch::Found { .. } => {}
        _ => panic!("PUT /users/123 should be found"),
    }
}

#[test]
fn test_multiple_router_composition_deep_nesting() {
    async fn handler() -> &'static str {
        "handler"
    }

    // Create nested routers at different depth levels
    let deep_router = Router::new().route("/action", get(handler));

    let mid_router = Router::new().route("/info", get(handler));

    let shallow_router = Router::new().route("/status", get(handler));

    // Nest at different depths
    let app = Router::new()
        .nest("/api/v1/resources/items", deep_router)
        .nest("/api/v1/resources", mid_router)
        .nest("/api", shallow_router);

    // Verify all routes are registered
    let routes = app.registered_routes();
    assert_eq!(routes.len(), 3, "Should have 3 routes registered");

    assert!(routes.contains_key("/api/v1/resources/items/action"));
    assert!(routes.contains_key("/api/v1/resources/info"));
    assert!(routes.contains_key("/api/status"));

    // Verify all routes are matchable
    match app.match_route("/api/v1/resources/items/action", &Method::GET) {
        RouteMatch::Found { .. } => {}
        _ => panic!("Should find deep route"),
    }
    match app.match_route("/api/v1/resources/info", &Method::GET) {
        RouteMatch::Found { .. } => {}
        _ => panic!("Should find mid route"),
    }
    match app.match_route("/api/status", &Method::GET) {
        RouteMatch::Found { .. } => {}
        _ => panic!("Should find shallow route"),
    }
}
