use super::{
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

mod property_tests {
    use crate::router::{
        convert_path_params, delete, get, normalize_path_for_comparison, normalize_prefix, patch,
        post, put, MethodRouter, RouteMatch, Router,
    };
    use http::Method;
    use proptest::prelude::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    // **Feature: router-nesting, Property 2: Prefix Normalization**
    //
    // For any prefix string (with or without leading/trailing slashes), the normalized
    // prefix should start with exactly one slash and have no trailing slash, and all
    // nested routes should have properly formed paths without double slashes.
    //
    // **Validates: Requirements 1.2, 1.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Normalized prefix always starts with exactly one slash
        ///
        /// For any input prefix, the normalized result should always start with
        /// exactly one leading slash.
        #[test]
        fn prop_normalized_prefix_starts_with_single_slash(
            // Generate prefix with optional leading slashes
            leading_slashes in prop::collection::vec(Just('/'), 0..5),
            segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 0..4),
            trailing_slashes in prop::collection::vec(Just('/'), 0..5),
        ) {
            // Build the input prefix
            let mut prefix = String::new();
            for _ in &leading_slashes {
                prefix.push('/');
            }
            for (i, segment) in segments.iter().enumerate() {
                if i > 0 {
                    prefix.push('/');
                }
                prefix.push_str(segment);
            }
            for _ in &trailing_slashes {
                prefix.push('/');
            }

            let normalized = normalize_prefix(&prefix);

            // Property 1: Always starts with exactly one slash
            prop_assert!(
                normalized.starts_with('/'),
                "Normalized prefix '{}' should start with '/', input was '{}'",
                normalized, prefix
            );

            // Property 2: No double slashes at the start
            prop_assert!(
                !normalized.starts_with("//"),
                "Normalized prefix '{}' should not start with '//', input was '{}'",
                normalized, prefix
            );
        }

        /// Property: Normalized prefix has no trailing slash (unless root)
        ///
        /// For any input prefix with non-empty segments, the normalized result
        /// should have no trailing slash.
        #[test]
        fn prop_normalized_prefix_no_trailing_slash(
            segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..4),
            trailing_slashes in prop::collection::vec(Just('/'), 0..5),
        ) {
            // Build the input prefix with segments
            let mut prefix = String::from("/");
            for (i, segment) in segments.iter().enumerate() {
                if i > 0 {
                    prefix.push('/');
                }
                prefix.push_str(segment);
            }
            for _ in &trailing_slashes {
                prefix.push('/');
            }

            let normalized = normalize_prefix(&prefix);

            // Property: No trailing slash when there are segments
            prop_assert!(
                !normalized.ends_with('/'),
                "Normalized prefix '{}' should not end with '/', input was '{}'",
                normalized, prefix
            );
        }

        /// Property: Normalized prefix has no double slashes
        ///
        /// For any input prefix, the normalized result should never contain
        /// consecutive slashes.
        #[test]
        fn prop_normalized_prefix_no_double_slashes(
            // Generate prefix with random slashes between segments
            segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..4),
            extra_slashes in prop::collection::vec(0..4usize, 1..4),
        ) {
            // Build the input prefix with extra slashes between segments
            let mut prefix = String::from("/");
            for (i, segment) in segments.iter().enumerate() {
                if i > 0 {
                    // Add extra slashes between segments
                    let num_slashes = extra_slashes.get(i).copied().unwrap_or(1);
                    for _ in 0..=num_slashes {
                        prefix.push('/');
                    }
                }
                prefix.push_str(segment);
            }

            let normalized = normalize_prefix(&prefix);

            // Property: No consecutive slashes
            prop_assert!(
                !normalized.contains("//"),
                "Normalized prefix '{}' should not contain '//', input was '{}'",
                normalized, prefix
            );
        }

        /// Property: Prefix normalization preserves segment content
        ///
        /// For any input prefix, all non-empty segments should be preserved
        /// in the normalized output in the same order.
        #[test]
        fn prop_normalized_prefix_preserves_segments(
            segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..4),
        ) {
            // Build the input prefix
            let prefix = format!("/{}", segments.join("/"));

            let normalized = normalize_prefix(&prefix);

            // Extract segments from normalized prefix
            let normalized_segments: Vec<&str> = normalized
                .split('/')
                .filter(|s| !s.is_empty())
                .collect();

            prop_assert_eq!(
                segments.len(),
                normalized_segments.len(),
                "Segment count should be preserved"
            );

            for (original, normalized_seg) in segments.iter().zip(normalized_segments.iter()) {
                prop_assert_eq!(
                    original, normalized_seg,
                    "Segment content should be preserved"
                );
            }
        }

        /// Property: Empty or slash-only input normalizes to root
        ///
        /// For any input that contains only slashes or is empty, the normalized
        /// result should be exactly "/".
        #[test]
        fn prop_empty_or_slashes_normalize_to_root(
            num_slashes in 0..10usize,
        ) {
            let prefix = "/".repeat(num_slashes);

            let normalized = normalize_prefix(&prefix);

            prop_assert_eq!(
                normalized, "/",
                "Empty or slash-only prefix '{}' should normalize to '/'",
                prefix
            );
        }
    }

    // **Feature: router-nesting, Property 3: HTTP Method Preservation**
    //
    // For any router with routes having multiple HTTP methods, cloning the MethodRouter
    // should preserve all method handlers for each route.
    //
    // **Validates: Requirements 1.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Cloning a MethodRouter preserves all HTTP method handlers
        ///
        /// For any combination of HTTP methods registered on a MethodRouter,
        /// cloning should preserve all handlers and their associated methods.
        #[test]
        fn prop_method_router_clone_preserves_methods(
            // Generate a random subset of HTTP methods to register
            use_get in any::<bool>(),
            use_post in any::<bool>(),
            use_put in any::<bool>(),
            use_patch in any::<bool>(),
            use_delete in any::<bool>(),
        ) {
            // Ensure at least one method is selected
            prop_assume!(use_get || use_post || use_put || use_patch || use_delete);

            // Build a MethodRouter with the selected methods
            let mut method_router = MethodRouter::new();
            let mut expected_methods: Vec<Method> = Vec::new();

            async fn handler() -> &'static str { "handler" }

            if use_get {
                method_router = get(handler);
                expected_methods.push(Method::GET);
            }

            if use_post {
                let post_router = post(handler);
                for (method, handler) in post_router.handlers {
                    method_router.handlers.insert(method.clone(), handler);
                    if !expected_methods.contains(&method) {
                        expected_methods.push(method);
                    }
                }
            }

            if use_put {
                let put_router = put(handler);
                for (method, handler) in put_router.handlers {
                    method_router.handlers.insert(method.clone(), handler);
                    if !expected_methods.contains(&method) {
                        expected_methods.push(method);
                    }
                }
            }

            if use_patch {
                let patch_router = patch(handler);
                for (method, handler) in patch_router.handlers {
                    method_router.handlers.insert(method.clone(), handler);
                    if !expected_methods.contains(&method) {
                        expected_methods.push(method);
                    }
                }
            }

            if use_delete {
                let delete_router = delete(handler);
                for (method, handler) in delete_router.handlers {
                    method_router.handlers.insert(method.clone(), handler);
                    if !expected_methods.contains(&method) {
                        expected_methods.push(method);
                    }
                }
            }

            // Clone the MethodRouter
            let cloned_router = method_router.clone();

            // Verify all methods are preserved in the clone
            let original_methods = method_router.allowed_methods();
            let cloned_methods = cloned_router.allowed_methods();

            prop_assert_eq!(
                original_methods.len(),
                cloned_methods.len(),
                "Cloned router should have same number of methods"
            );

            for method in &expected_methods {
                prop_assert!(
                    cloned_router.get_handler(method).is_some(),
                    "Cloned router should have handler for method {:?}",
                    method
                );
            }

            // Verify handlers are accessible (not null/invalid)
            for method in &cloned_methods {
                prop_assert!(
                    cloned_router.get_handler(method).is_some(),
                    "Handler for {:?} should be accessible after clone",
                    method
                );
            }
        }
    }

    // **Feature: router-nesting, Property 1: Route Registration with Prefix**
    //
    // For any router with routes and any valid prefix, nesting the router should
    // result in all routes being registered with the prefix prepended to their
    // original paths.
    //
    // **Validates: Requirements 1.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: All nested routes are registered with prefix prepended
        ///
        /// For any router with routes and any valid prefix, nesting should result
        /// in all routes being registered with the prefix prepended.
        #[test]
        fn prop_nested_routes_have_prefix(
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

            // Create nested router and nest it
            let nested_router = Router::new().route(&route_path, get(handler));
            let app = Router::new().nest(&prefix, nested_router);

            // Build expected prefixed path (matchit format)
            let expected_matchit_path = if has_param {
                format!("{}/{}/:id", prefix, route_segments.join("/"))
            } else {
                format!("{}/{}", prefix, route_segments.join("/"))
            };

            let routes = app.registered_routes();

            // Property: The prefixed route should exist
            prop_assert!(
                routes.contains_key(&expected_matchit_path),
                "Expected route '{}' not found. Available routes: {:?}",
                expected_matchit_path,
                routes.keys().collect::<Vec<_>>()
            );

            // Property: The route info should have the correct display path
            let route_info = routes.get(&expected_matchit_path).unwrap();
            let expected_display_path = format!("{}{}", prefix, route_path);
            prop_assert_eq!(
                &route_info.path, &expected_display_path,
                "Display path should be prefix + original path"
            );
        }

        /// Property: Number of routes is preserved after nesting
        ///
        /// For any router with N routes, nesting should result in exactly N routes
        /// being registered in the parent router (assuming no conflicts).
        #[test]
        fn prop_route_count_preserved_after_nesting(
            // Generate number of routes (1-3 to keep test fast)
            num_routes in 1..4usize,
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
        ) {
            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix_segments.join("/"));

            // Create nested router with multiple routes
            let mut nested_router = Router::new();
            for i in 0..num_routes {
                let path = format!("/route{}", i);
                nested_router = nested_router.route(&path, get(handler));
            }

            let app = Router::new().nest(&prefix, nested_router);

            prop_assert_eq!(
                app.registered_routes().len(),
                num_routes,
                "Number of routes should be preserved after nesting"
            );
        }

        /// Property: Nested routes are matchable
        ///
        /// For any nested route, a request to the prefixed path should match.
        #[test]
        fn prop_nested_routes_are_matchable(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
            route_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
        ) {
            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix_segments.join("/"));
            let route_path = format!("/{}", route_segments.join("/"));

            let nested_router = Router::new().route(&route_path, get(handler));
            let app = Router::new().nest(&prefix, nested_router);

            // Build the full path to match
            let full_path = format!("{}{}", prefix, route_path);

            // Property: The route should be matchable
            match app.match_route(&full_path, &Method::GET) {
                RouteMatch::Found { .. } => {
                    // Success - route was found
                }
                RouteMatch::NotFound => {
                    prop_assert!(false, "Route '{}' should be found but got NotFound", full_path);
                }
                RouteMatch::MethodNotAllowed { .. } => {
                    prop_assert!(false, "Route '{}' should be found but got MethodNotAllowed", full_path);
                }
            }
        }
    }

    // **Feature: router-nesting, Property 9: State Merging**
    //
    // For any nested router with state, that state should be accessible via the
    // State extractor in handlers after nesting (assuming no type conflict with parent).
    //
    // **Validates: Requirements 3.1, 3.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: State type IDs are merged from nested router
        ///
        /// For any nested router with state, the parent router should track
        /// the state type IDs after nesting.
        #[test]
        fn prop_state_type_ids_merged(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
            has_nested_state in any::<bool>(),
        ) {
            #[derive(Clone)]
            struct TestState(#[allow(dead_code)] i32);

            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix_segments.join("/"));

            let mut nested = Router::new().route("/test", get(handler));
            if has_nested_state {
                nested = nested.state(TestState(42));
            }

            let parent = Router::new().nest(&prefix, nested);

            // Property: If nested had state, parent should track the type ID
            if has_nested_state {
                prop_assert!(
                    parent.state_type_ids().contains(&std::any::TypeId::of::<TestState>()),
                    "Parent should track nested state type ID"
                );
            }
        }

        /// Property: State merging adds nested state to parent
        ///
        /// For any nested router with state that the parent doesn't have,
        /// merge_state should add that state to the parent.
        #[test]
        fn prop_merge_state_adds_nested_state(
            state_value in any::<i32>(),
        ) {
            #[derive(Clone, PartialEq, Debug)]
            struct UniqueState(i32);

            // Create a source router with state
            let source = Router::new().state(UniqueState(state_value));

            // Create a parent without this state type
            let parent = Router::new().merge_state::<UniqueState>(&source);

            // Property: Parent should now have the state
            prop_assert!(
                parent.has_state::<UniqueState>(),
                "Parent should have state after merge"
            );

            // Property: State value should match
            let merged_state = parent.state.get::<UniqueState>().unwrap();
            prop_assert_eq!(
                merged_state.0, state_value,
                "Merged state value should match source"
            );
        }
    }

    // **Feature: router-nesting, Property 10: State Precedence**
    //
    // For any parent and nested router both having state of the same type,
    // the parent's state value should be preserved after nesting.
    //
    // **Validates: Requirements 3.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Parent state takes precedence over nested state
        ///
        /// For any parent and nested router both having state of the same type,
        /// the parent's state value should be preserved after merge_state.
        #[test]
        fn prop_parent_state_takes_precedence(
            parent_value in any::<i32>(),
            nested_value in any::<i32>(),
        ) {
            // Ensure values are different to make the test meaningful
            prop_assume!(parent_value != nested_value);

            #[derive(Clone, PartialEq, Debug)]
            struct SharedState(i32);

            // Create source router with nested state
            let source = Router::new().state(SharedState(nested_value));

            // Create parent with its own state
            let parent = Router::new()
                .state(SharedState(parent_value))
                .merge_state::<SharedState>(&source);

            // Property: Parent should still have state
            prop_assert!(
                parent.has_state::<SharedState>(),
                "Parent should have state"
            );

            // Property: Parent's state value should be preserved (parent wins)
            let final_state = parent.state.get::<SharedState>().unwrap();
            prop_assert_eq!(
                final_state.0, parent_value,
                "Parent state value should be preserved, not overwritten by nested"
            );
        }

        /// Property: State precedence is consistent regardless of merge order
        ///
        /// For any parent with state, merging from a source with the same type
        /// should always preserve the parent's value.
        #[test]
        fn prop_state_precedence_consistent(
            parent_value in any::<i32>(),
            source1_value in any::<i32>(),
            source2_value in any::<i32>(),
        ) {
            #[derive(Clone, PartialEq, Debug)]
            struct ConsistentState(i32);

            // Create multiple source routers
            let source1 = Router::new().state(ConsistentState(source1_value));
            let source2 = Router::new().state(ConsistentState(source2_value));

            // Create parent with its own state and merge from multiple sources
            let parent = Router::new()
                .state(ConsistentState(parent_value))
                .merge_state::<ConsistentState>(&source1)
                .merge_state::<ConsistentState>(&source2);

            // Property: Parent's original state should be preserved
            let final_state = parent.state.get::<ConsistentState>().unwrap();
            prop_assert_eq!(
                final_state.0, parent_value,
                "Parent state should be preserved after multiple merges"
            );
        }
    }

    // **Feature: phase4-ergonomics-v1, Property 1: Route Conflict Detection**
    //
    // For any two routes with the same path and HTTP method registered on the same
    // RustApi instance, the system should detect the conflict and report an error
    // at startup time.
    //
    // **Validates: Requirements 1.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Routes with identical path structure but different parameter names conflict
        ///
        /// For any valid path with parameters, registering two routes with the same
        /// structure but different parameter names should be detected as a conflict.
        #[test]
        fn prop_same_structure_different_param_names_conflict(
            // Generate valid path segments
            segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..4),
            // Generate two different parameter names
            param1 in "[a-z][a-z0-9]{0,5}",
            param2 in "[a-z][a-z0-9]{0,5}",
        ) {
            // Ensure param names are different
            prop_assume!(param1 != param2);

            // Build two paths with same structure but different param names
            let mut path1 = String::from("/");
            let mut path2 = String::from("/");

            for segment in &segments {
                path1.push_str(segment);
                path1.push('/');
                path2.push_str(segment);
                path2.push('/');
            }

            path1.push('{');
            path1.push_str(&param1);
            path1.push('}');

            path2.push('{');
            path2.push_str(&param2);
            path2.push('}');

            // Try to register both routes - should panic
            let result = catch_unwind(AssertUnwindSafe(|| {
                async fn handler1() -> &'static str { "handler1" }
                async fn handler2() -> &'static str { "handler2" }

                let _router = Router::new()
                    .route(&path1, get(handler1))
                    .route(&path2, get(handler2));
            }));

            prop_assert!(
                result.is_err(),
                "Routes '{}' and '{}' should conflict but didn't",
                path1, path2
            );
        }

        /// Property: Routes with different path structures don't conflict
        ///
        /// For any two paths with different structures (different number of segments
        /// or different static segments), they should not conflict.
        #[test]
        fn prop_different_structures_no_conflict(
            // Generate different path segments for two routes
            segments1 in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
            segments2 in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..3),
            // Optional parameter at the end
            has_param1 in any::<bool>(),
            has_param2 in any::<bool>(),
        ) {
            // Build two paths
            let mut path1 = String::from("/");
            let mut path2 = String::from("/");

            for segment in &segments1 {
                path1.push_str(segment);
                path1.push('/');
            }
            path1.pop(); // Remove trailing slash

            for segment in &segments2 {
                path2.push_str(segment);
                path2.push('/');
            }
            path2.pop(); // Remove trailing slash

            if has_param1 {
                path1.push_str("/{id}");
            }

            if has_param2 {
                path2.push_str("/{id}");
            }

            // Normalize paths for comparison
            let norm1 = normalize_path_for_comparison(&convert_path_params(&path1));
            let norm2 = normalize_path_for_comparison(&convert_path_params(&path2));

            // Only test if paths are actually different
            prop_assume!(norm1 != norm2);

            // Try to register both routes - should succeed
            let result = catch_unwind(AssertUnwindSafe(|| {
                async fn handler1() -> &'static str { "handler1" }
                async fn handler2() -> &'static str { "handler2" }

                let router = Router::new()
                    .route(&path1, get(handler1))
                    .route(&path2, get(handler2));

                router.registered_routes().len()
            }));

            prop_assert!(
                result.is_ok(),
                "Routes '{}' and '{}' should not conflict but did",
                path1, path2
            );

            if let Ok(count) = result {
                prop_assert_eq!(count, 2, "Should have registered 2 routes");
            }
        }

        /// Property: Conflict error message contains both route paths
        ///
        /// When a conflict is detected, the error message should include both
        /// the existing route path and the new conflicting route path.
        #[test]
        fn prop_conflict_error_contains_both_paths(
            // Generate a valid path segment
            segment in "[a-z][a-z0-9]{1,5}",
            param1 in "[a-z][a-z0-9]{1,5}",
            param2 in "[a-z][a-z0-9]{1,5}",
        ) {
            prop_assume!(param1 != param2);

            let path1 = format!("/{}/{{{}}}", segment, param1);
            let path2 = format!("/{}/{{{}}}", segment, param2);

            let result = catch_unwind(AssertUnwindSafe(|| {
                async fn handler1() -> &'static str { "handler1" }
                async fn handler2() -> &'static str { "handler2" }

                let _router = Router::new()
                    .route(&path1, get(handler1))
                    .route(&path2, get(handler2));
            }));

            prop_assert!(result.is_err(), "Should have panicked due to conflict");

            // Check that the panic message contains useful information
            if let Err(panic_info) = result {
                if let Some(msg) = panic_info.downcast_ref::<String>() {
                    prop_assert!(
                        msg.contains("ROUTE CONFLICT DETECTED"),
                        "Error should contain 'ROUTE CONFLICT DETECTED', got: {}",
                        msg
                    );
                    prop_assert!(
                        msg.contains("Existing:") && msg.contains("New:"),
                        "Error should contain both 'Existing:' and 'New:' labels, got: {}",
                        msg
                    );
                    prop_assert!(
                        msg.contains("How to resolve:"),
                        "Error should contain resolution guidance, got: {}",
                        msg
                    );
                }
            }
        }

        /// Property: Exact duplicate paths conflict
        ///
        /// Registering the exact same path twice should always be detected as a conflict.
        #[test]
        fn prop_exact_duplicate_paths_conflict(
            // Generate valid path segments
            segments in prop::collection::vec("[a-z][a-z0-9]{0,5}", 1..4),
            has_param in any::<bool>(),
        ) {
            // Build a path
            let mut path = String::from("/");

            for segment in &segments {
                path.push_str(segment);
                path.push('/');
            }
            path.pop(); // Remove trailing slash

            if has_param {
                path.push_str("/{id}");
            }

            // Try to register the same path twice - should panic
            let result = catch_unwind(AssertUnwindSafe(|| {
                async fn handler1() -> &'static str { "handler1" }
                async fn handler2() -> &'static str { "handler2" }

                let _router = Router::new()
                    .route(&path, get(handler1))
                    .route(&path, get(handler2));
            }));

            prop_assert!(
                result.is_err(),
                "Registering path '{}' twice should conflict but didn't",
                path
            );
        }
    }

    // **Feature: router-nesting, Property 5: Nested Route Matching**
    //
    // For any nested route and a request with a matching path and method,
    // the router should return the correct handler.
    //
    // **Validates: Requirements 2.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Nested routes with path parameters are correctly matched
        ///
        /// For any nested route with path parameters, a request to the prefixed path
        /// with valid parameter values should match and return Found.
        #[test]
        fn prop_nested_route_with_params_matches(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
            route_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 0..2),
            param_value in "[a-z0-9]{1,10}",
        ) {
            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix_segments.join("/"));
            let route_path = if route_segments.is_empty() {
                "/{id}".to_string()
            } else {
                format!("/{}/{{id}}", route_segments.join("/"))
            };

            let nested_router = Router::new().route(&route_path, get(handler));
            let app = Router::new().nest(&prefix, nested_router);

            // Build the full path to match with actual parameter value
            let full_path = if route_segments.is_empty() {
                format!("{}/{}", prefix, param_value)
            } else {
                format!("{}/{}/{}", prefix, route_segments.join("/"), param_value)
            };

            // Property: The route should be matched
            match app.match_route(&full_path, &Method::GET) {
                RouteMatch::Found { params, .. } => {
                    // Verify the parameter was extracted
                    prop_assert!(
                        params.contains_key("id"),
                        "Should have 'id' parameter, got: {:?}",
                        params
                    );
                    prop_assert_eq!(
                        params.get("id").unwrap(),
                        &param_value,
                        "Parameter value should match"
                    );
                }
                RouteMatch::NotFound => {
                    prop_assert!(false, "Route '{}' should be found but got NotFound", full_path);
                }
                RouteMatch::MethodNotAllowed { .. } => {
                    prop_assert!(false, "Route '{}' should be found but got MethodNotAllowed", full_path);
                }
            }
        }

        /// Property: Nested routes match correct HTTP method
        ///
        /// For any nested route registered with a specific HTTP method, only requests
        /// with that method should return Found.
        #[test]
        fn prop_nested_route_matches_correct_method(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..2),
            route_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..2),
            use_get in any::<bool>(),
        ) {
            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix_segments.join("/"));
            let route_path = format!("/{}", route_segments.join("/"));

            // Register with either GET or POST
            let method_router = if use_get { get(handler) } else { post(handler) };
            let nested_router = Router::new().route(&route_path, method_router);
            let app = Router::new().nest(&prefix, nested_router);

            let full_path = format!("{}{}", prefix, route_path);
            let registered_method = if use_get { Method::GET } else { Method::POST };
            let other_method = if use_get { Method::POST } else { Method::GET };

            // Property: Registered method should match
            match app.match_route(&full_path, &registered_method) {
                RouteMatch::Found { .. } => {
                    // Success
                }
                other => {
                    prop_assert!(false, "Route should be found for registered method, got: {:?}",
                        match other {
                            RouteMatch::NotFound => "NotFound",
                            RouteMatch::MethodNotAllowed { .. } => "MethodNotAllowed",
                            _ => "Found",
                        }
                    );
                }
            }

            // Property: Other method should return MethodNotAllowed
            match app.match_route(&full_path, &other_method) {
                RouteMatch::MethodNotAllowed { allowed } => {
                    prop_assert!(
                        allowed.contains(&registered_method),
                        "Allowed methods should contain {:?}",
                        registered_method
                    );
                }
                other => {
                    prop_assert!(false, "Route should return MethodNotAllowed for other method, got: {:?}",
                        match other {
                            RouteMatch::NotFound => "NotFound",
                            RouteMatch::Found { .. } => "Found",
                            _ => "MethodNotAllowed",
                        }
                    );
                }
            }
        }
    }

    // **Feature: router-nesting, Property 6: Path Parameter Extraction**
    //
    // For any nested route with path parameters and a matching request,
    // the extracted parameters should have the correct names and values.
    //
    // **Validates: Requirements 2.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Single path parameter is correctly extracted from nested route
        ///
        /// For any nested route with a single path parameter, the parameter name
        /// and value should be correctly extracted.
        #[test]
        fn prop_single_param_extraction(
            prefix in "[a-z][a-z0-9]{1,5}",
            param_name in "[a-z][a-z0-9]{1,5}",
            param_value in "[a-z0-9]{1,10}",
        ) {
            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix);
            let route_path = format!("/{{{}}}", param_name);

            let nested_router = Router::new().route(&route_path, get(handler));
            let app = Router::new().nest(&prefix, nested_router);

            let full_path = format!("{}/{}", prefix, param_value);

            match app.match_route(&full_path, &Method::GET) {
                RouteMatch::Found { params, .. } => {
                    prop_assert!(
                        params.contains_key(&param_name),
                        "Should have '{}' parameter, got: {:?}",
                        param_name, params
                    );
                    prop_assert_eq!(
                        params.get(&param_name).unwrap(),
                        &param_value,
                        "Parameter '{}' value should be '{}'",
                        param_name, param_value
                    );
                }
                _ => {
                    prop_assert!(false, "Route should be found");
                }
            }
        }

        /// Property: Multiple path parameters are correctly extracted from nested route
        ///
        /// For any nested route with multiple path parameters, all parameters
        /// should be correctly extracted with their names and values.
        #[test]
        fn prop_multiple_params_extraction(
            prefix in "[a-z][a-z0-9]{1,5}",
            param1_name in "[a-z]{1,5}",
            param1_value in "[a-z0-9]{1,10}",
            param2_name in "[a-z]{1,5}",
            param2_value in "[a-z0-9]{1,10}",
        ) {
            // Ensure param names are different
            prop_assume!(param1_name != param2_name);

            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix);
            let route_path = format!("/{{{}}}/items/{{{}}}", param1_name, param2_name);

            let nested_router = Router::new().route(&route_path, get(handler));
            let app = Router::new().nest(&prefix, nested_router);

            let full_path = format!("{}/{}/items/{}", prefix, param1_value, param2_value);

            match app.match_route(&full_path, &Method::GET) {
                RouteMatch::Found { params, .. } => {
                    // Check first parameter
                    prop_assert!(
                        params.contains_key(&param1_name),
                        "Should have '{}' parameter, got: {:?}",
                        param1_name, params
                    );
                    prop_assert_eq!(
                        params.get(&param1_name).unwrap(),
                        &param1_value,
                        "Parameter '{}' value should be '{}'",
                        param1_name, param1_value
                    );

                    // Check second parameter
                    prop_assert!(
                        params.contains_key(&param2_name),
                        "Should have '{}' parameter, got: {:?}",
                        param2_name, params
                    );
                    prop_assert_eq!(
                        params.get(&param2_name).unwrap(),
                        &param2_value,
                        "Parameter '{}' value should be '{}'",
                        param2_name, param2_value
                    );
                }
                _ => {
                    prop_assert!(false, "Route should be found");
                }
            }
        }

        /// Property: Path parameters preserve special characters in values
        ///
        /// For any nested route with path parameters, parameter values containing
        /// URL-safe special characters should be preserved correctly.
        #[test]
        fn prop_param_value_preservation(
            prefix in "[a-z]{1,5}",
            // Generate values with alphanumeric and some special chars
            param_value in "[a-zA-Z0-9_-]{1,15}",
        ) {
            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix);
            let route_path = "/{id}".to_string();

            let nested_router = Router::new().route(&route_path, get(handler));
            let app = Router::new().nest(&prefix, nested_router);

            let full_path = format!("{}/{}", prefix, param_value);

            match app.match_route(&full_path, &Method::GET) {
                RouteMatch::Found { params, .. } => {
                    prop_assert_eq!(
                        params.get("id").unwrap(),
                        &param_value,
                        "Parameter value should be preserved exactly"
                    );
                }
                _ => {
                    prop_assert!(false, "Route should be found");
                }
            }
        }
    }

    // **Feature: router-nesting, Property 7: Not Found Response**
    //
    // For any request path that doesn't match any registered route (nested or otherwise),
    // the router should return NotFound.
    //
    // **Validates: Requirements 2.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Unregistered paths return NotFound
        ///
        /// For any path that doesn't match any registered route, the router
        /// should return NotFound.
        #[test]
        fn prop_unregistered_path_returns_not_found(
            prefix in "[a-z][a-z0-9]{1,5}",
            route_segment in "[a-z][a-z0-9]{1,5}",
            unregistered_segment in "[a-z][a-z0-9]{6,10}",
        ) {
            // Ensure segments are different
            prop_assume!(route_segment != unregistered_segment);

            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix);
            let route_path = format!("/{}", route_segment);

            let nested_router = Router::new().route(&route_path, get(handler));
            let app = Router::new().nest(&prefix, nested_router);

            // Try to match an unregistered path
            let unregistered_path = format!("{}/{}", prefix, unregistered_segment);

            match app.match_route(&unregistered_path, &Method::GET) {
                RouteMatch::NotFound => {
                    // Success - this is expected
                }
                RouteMatch::Found { .. } => {
                    prop_assert!(false, "Path '{}' should not be found", unregistered_path);
                }
                RouteMatch::MethodNotAllowed { .. } => {
                    prop_assert!(false, "Path '{}' should return NotFound, not MethodNotAllowed", unregistered_path);
                }
            }
        }

        /// Property: Wrong prefix returns NotFound
        ///
        /// For any nested route, a request with a different prefix should return NotFound.
        #[test]
        fn prop_wrong_prefix_returns_not_found(
            prefix1 in "[a-z][a-z0-9]{1,5}",
            prefix2 in "[a-z][a-z0-9]{6,10}",
            route_segment in "[a-z][a-z0-9]{1,5}",
        ) {
            // Ensure prefixes are different
            prop_assume!(prefix1 != prefix2);

            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix1);
            let route_path = format!("/{}", route_segment);

            let nested_router = Router::new().route(&route_path, get(handler));
            let app = Router::new().nest(&prefix, nested_router);

            // Try to match with wrong prefix
            let wrong_prefix_path = format!("/{}/{}", prefix2, route_segment);

            match app.match_route(&wrong_prefix_path, &Method::GET) {
                RouteMatch::NotFound => {
                    // Success - this is expected
                }
                _ => {
                    prop_assert!(false, "Path '{}' with wrong prefix should return NotFound", wrong_prefix_path);
                }
            }
        }

        /// Property: Partial path match returns NotFound
        ///
        /// For any nested route with multiple segments, a request matching only
        /// part of the path should return NotFound.
        #[test]
        fn prop_partial_path_returns_not_found(
            prefix in "[a-z][a-z0-9]{1,5}",
            segment1 in "[a-z][a-z0-9]{1,5}",
            segment2 in "[a-z][a-z0-9]{1,5}",
        ) {
            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix);
            let route_path = format!("/{}/{}", segment1, segment2);

            let nested_router = Router::new().route(&route_path, get(handler));
            let app = Router::new().nest(&prefix, nested_router);

            // Try to match only the first segment (partial path)
            let partial_path = format!("{}/{}", prefix, segment1);

            match app.match_route(&partial_path, &Method::GET) {
                RouteMatch::NotFound => {
                    // Success - partial path should not match
                }
                _ => {
                    prop_assert!(false, "Partial path '{}' should return NotFound", partial_path);
                }
            }
        }
    }

    // **Feature: router-nesting, Property 8: Method Not Allowed Response**
    //
    // For any request to a valid path but with an unregistered HTTP method,
    // the router should return MethodNotAllowed with the list of allowed methods.
    //
    // **Validates: Requirements 2.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Unregistered method returns MethodNotAllowed with allowed methods
        ///
        /// For any nested route registered with specific methods, a request with
        /// an unregistered method should return MethodNotAllowed with the correct
        /// list of allowed methods.
        #[test]
        fn prop_unregistered_method_returns_method_not_allowed(
            prefix in "[a-z][a-z0-9]{1,5}",
            route_segment in "[a-z][a-z0-9]{1,5}",
        ) {
            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix);
            let route_path = format!("/{}", route_segment);

            // Register only GET
            let nested_router = Router::new().route(&route_path, get(handler));
            let app = Router::new().nest(&prefix, nested_router);

            let full_path = format!("{}{}", prefix, route_path);

            // Try POST on a GET-only route
            match app.match_route(&full_path, &Method::POST) {
                RouteMatch::MethodNotAllowed { allowed } => {
                    prop_assert!(
                        allowed.contains(&Method::GET),
                        "Allowed methods should contain GET, got: {:?}",
                        allowed
                    );
                    prop_assert!(
                        !allowed.contains(&Method::POST),
                        "Allowed methods should not contain POST"
                    );
                }
                RouteMatch::Found { .. } => {
                    prop_assert!(false, "POST should not be found on GET-only route");
                }
                RouteMatch::NotFound => {
                    prop_assert!(false, "Path exists, should return MethodNotAllowed not NotFound");
                }
            }
        }

        /// Property: Multiple registered methods are all returned in allowed list
        ///
        /// For any nested route registered with multiple methods, the MethodNotAllowed
        /// response should include all registered methods.
        #[test]
        fn prop_multiple_methods_in_allowed_list(
            prefix in "[a-z][a-z0-9]{1,5}",
            route_segment in "[a-z][a-z0-9]{1,5}",
            use_get in any::<bool>(),
            use_post in any::<bool>(),
            use_put in any::<bool>(),
        ) {
            // Ensure at least one method is registered
            prop_assume!(use_get || use_post || use_put);

            async fn handler() -> &'static str { "handler" }

            let prefix = format!("/{}", prefix);
            let route_path = format!("/{}", route_segment);

            // Build method router with selected methods
            let mut method_router = MethodRouter::new();
            let mut expected_methods: Vec<Method> = Vec::new();

            if use_get {
                let get_router = get(handler);
                for (method, h) in get_router.handlers {
                    method_router.handlers.insert(method.clone(), h);
                    expected_methods.push(method);
                }
            }
            if use_post {
                let post_router = post(handler);
                for (method, h) in post_router.handlers {
                    method_router.handlers.insert(method.clone(), h);
                    expected_methods.push(method);
                }
            }
            if use_put {
                let put_router = put(handler);
                for (method, h) in put_router.handlers {
                    method_router.handlers.insert(method.clone(), h);
                    expected_methods.push(method);
                }
            }

            let nested_router = Router::new().route(&route_path, method_router);
            let app = Router::new().nest(&prefix, nested_router);

            let full_path = format!("{}{}", prefix, route_path);

            // Try DELETE (which we never register)
            match app.match_route(&full_path, &Method::DELETE) {
                RouteMatch::MethodNotAllowed { allowed } => {
                    // All registered methods should be in allowed list
                    for method in &expected_methods {
                        prop_assert!(
                            allowed.contains(method),
                            "Allowed methods should contain {:?}, got: {:?}",
                            method, allowed
                        );
                    }
                    // DELETE should not be in allowed list
                    prop_assert!(
                        !allowed.contains(&Method::DELETE),
                        "Allowed methods should not contain DELETE"
                    );
                }
                RouteMatch::Found { .. } => {
                    prop_assert!(false, "DELETE should not be found");
                }
                RouteMatch::NotFound => {
                    prop_assert!(false, "Path exists, should return MethodNotAllowed not NotFound");
                }
            }
        }
    }

    // **Feature: router-nesting, Property 12: Conflict Detection**
    //
    // For any nested route that conflicts with an existing route (same path structure),
    // the router should detect and report the conflict with both route paths.
    //
    // **Validates: Requirements 5.1, 5.3**

    // **Feature: router-nesting, Property 4: Multiple Router Composition**
    //
    // For any set of routers with non-overlapping route structures nested under
    // different prefixes, all routes should be registered without conflicts.
    //
    // **Validates: Requirements 1.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Multiple routers nested under different prefixes register all routes
        ///
        /// For any set of routers with routes nested under different prefixes,
        /// all routes should be registered and the total count should equal the
        /// sum of routes from all nested routers.
        #[test]
        fn prop_multiple_routers_all_routes_registered(
            // Generate 2-3 different prefixes
            prefix1_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
            prefix2_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
            // Generate route counts for each router (1-3 routes each)
            num_routes1 in 1..4usize,
            num_routes2 in 1..4usize,
        ) {
            // Build prefixes
            let prefix1 = format!("/{}", prefix1_segments.join("/"));
            let prefix2 = format!("/{}", prefix2_segments.join("/"));

            // Ensure prefixes are different
            prop_assume!(prefix1 != prefix2);

            async fn handler() -> &'static str { "handler" }

            // Create first router with routes
            let mut router1 = Router::new();
            for i in 0..num_routes1 {
                let path = format!("/route1_{}", i);
                router1 = router1.route(&path, get(handler));
            }

            // Create second router with routes
            let mut router2 = Router::new();
            for i in 0..num_routes2 {
                let path = format!("/route2_{}", i);
                router2 = router2.route(&path, get(handler));
            }

            // Nest both routers
            let app = Router::new()
                .nest(&prefix1, router1)
                .nest(&prefix2, router2);

            let routes = app.registered_routes();

            // Property: Total route count should equal sum of all nested routes
            let expected_count = num_routes1 + num_routes2;
            prop_assert_eq!(
                routes.len(),
                expected_count,
                "Should have {} routes ({}+{}), got {}",
                expected_count, num_routes1, num_routes2, routes.len()
            );

            // Property: All routes from router1 should be registered with prefix1
            for i in 0..num_routes1 {
                let expected_path = format!("{}/route1_{}", prefix1, i);
                let matchit_path = convert_path_params(&expected_path);
                prop_assert!(
                    routes.contains_key(&matchit_path),
                    "Route '{}' should be registered",
                    expected_path
                );
            }

            // Property: All routes from router2 should be registered with prefix2
            for i in 0..num_routes2 {
                let expected_path = format!("{}/route2_{}", prefix2, i);
                let matchit_path = convert_path_params(&expected_path);
                prop_assert!(
                    routes.contains_key(&matchit_path),
                    "Route '{}' should be registered",
                    expected_path
                );
            }
        }

        /// Property: Multiple routers with same internal routes don't interfere
        ///
        /// For any set of routers with identical internal route structures nested
        /// under different prefixes, all routes should be independently matchable.
        #[test]
        fn prop_multiple_routers_no_interference(
            prefix1 in "[a-z][a-z0-9]{1,5}",
            prefix2 in "[a-z][a-z0-9]{1,5}",
            route_segment in "[a-z][a-z0-9]{1,5}",
            param_value1 in "[a-z0-9]{1,10}",
            param_value2 in "[a-z0-9]{1,10}",
        ) {
            // Ensure prefixes are different
            prop_assume!(prefix1 != prefix2);

            let prefix1 = format!("/{}", prefix1);
            let prefix2 = format!("/{}", prefix2);

            async fn handler() -> &'static str { "handler" }

            // Create two routers with identical internal structure
            let router1 = Router::new()
                .route(&format!("/{}", route_segment), get(handler))
                .route("/{id}", get(handler));

            let router2 = Router::new()
                .route(&format!("/{}", route_segment), get(handler))
                .route("/{id}", get(handler));

            // Nest both routers
            let app = Router::new()
                .nest(&prefix1, router1)
                .nest(&prefix2, router2);

            // Property: Routes under prefix1 should be matchable
            let path1_static = format!("{}/{}", prefix1, route_segment);
            match app.match_route(&path1_static, &Method::GET) {
                RouteMatch::Found { params, .. } => {
                    prop_assert!(params.is_empty(), "Static path should have no params");
                }
                _ => {
                    prop_assert!(false, "Route '{}' should be found", path1_static);
                }
            }

            let path1_param = format!("{}/{}", prefix1, param_value1);
            match app.match_route(&path1_param, &Method::GET) {
                RouteMatch::Found { params, .. } => {
                    prop_assert_eq!(
                        params.get("id"),
                        Some(&param_value1.to_string()),
                        "Parameter should be extracted correctly"
                    );
                }
                _ => {
                    prop_assert!(false, "Route '{}' should be found", path1_param);
                }
            }

            // Property: Routes under prefix2 should be matchable independently
            let path2_static = format!("{}/{}", prefix2, route_segment);
            match app.match_route(&path2_static, &Method::GET) {
                RouteMatch::Found { params, .. } => {
                    prop_assert!(params.is_empty(), "Static path should have no params");
                }
                _ => {
                    prop_assert!(false, "Route '{}' should be found", path2_static);
                }
            }

            let path2_param = format!("{}/{}", prefix2, param_value2);
            match app.match_route(&path2_param, &Method::GET) {
                RouteMatch::Found { params, .. } => {
                    prop_assert_eq!(
                        params.get("id"),
                        Some(&param_value2.to_string()),
                        "Parameter should be extracted correctly"
                    );
                }
                _ => {
                    prop_assert!(false, "Route '{}' should be found", path2_param);
                }
            }
        }

        /// Property: Multiple routers preserve HTTP methods independently
        ///
        /// For any set of routers with different HTTP methods nested under different
        /// prefixes, each route should preserve its own set of allowed methods.
        #[test]
        fn prop_multiple_routers_preserve_methods(
            prefix1 in "[a-z][a-z0-9]{1,5}",
            prefix2 in "[a-z][a-z0-9]{1,5}",
            route_segment in "[a-z][a-z0-9]{1,5}",
            router1_use_get in any::<bool>(),
            router1_use_post in any::<bool>(),
            router2_use_get in any::<bool>(),
            router2_use_put in any::<bool>(),
        ) {
            // Ensure at least one method per router
            prop_assume!(router1_use_get || router1_use_post);
            prop_assume!(router2_use_get || router2_use_put);
            // Ensure prefixes are different
            prop_assume!(prefix1 != prefix2);

            let prefix1 = format!("/{}", prefix1);
            let prefix2 = format!("/{}", prefix2);
            let route_path = format!("/{}", route_segment);

            async fn handler() -> &'static str { "handler" }

            // Build router1 with selected methods
            let mut method_router1 = MethodRouter::new();
            let mut expected_methods1: Vec<Method> = Vec::new();
            if router1_use_get {
                let get_router = get(handler);
                for (method, h) in get_router.handlers {
                    method_router1.handlers.insert(method.clone(), h);
                    expected_methods1.push(method);
                }
            }
            if router1_use_post {
                let post_router = post(handler);
                for (method, h) in post_router.handlers {
                    method_router1.handlers.insert(method.clone(), h);
                    expected_methods1.push(method);
                }
            }

            // Build router2 with selected methods
            let mut method_router2 = MethodRouter::new();
            let mut expected_methods2: Vec<Method> = Vec::new();
            if router2_use_get {
                let get_router = get(handler);
                for (method, h) in get_router.handlers {
                    method_router2.handlers.insert(method.clone(), h);
                    expected_methods2.push(method);
                }
            }
            if router2_use_put {
                let put_router = put(handler);
                for (method, h) in put_router.handlers {
                    method_router2.handlers.insert(method.clone(), h);
                    expected_methods2.push(method);
                }
            }

            let router1 = Router::new().route(&route_path, method_router1);
            let router2 = Router::new().route(&route_path, method_router2);

            let app = Router::new()
                .nest(&prefix1, router1)
                .nest(&prefix2, router2);

            let full_path1 = format!("{}{}", prefix1, route_path);
            let full_path2 = format!("{}{}", prefix2, route_path);

            // Property: Router1's methods should be preserved
            for method in &expected_methods1 {
                match app.match_route(&full_path1, method) {
                    RouteMatch::Found { .. } => {}
                    _ => {
                        prop_assert!(false, "Method {:?} should be found for {}", method, full_path1);
                    }
                }
            }

            // Property: Router2's methods should be preserved
            for method in &expected_methods2 {
                match app.match_route(&full_path2, method) {
                    RouteMatch::Found { .. } => {}
                    _ => {
                        prop_assert!(false, "Method {:?} should be found for {}", method, full_path2);
                    }
                }
            }

            // Property: Methods not registered should return MethodNotAllowed
            if !expected_methods1.contains(&Method::DELETE) {
                match app.match_route(&full_path1, &Method::DELETE) {
                    RouteMatch::MethodNotAllowed { allowed } => {
                        for method in &expected_methods1 {
                            prop_assert!(
                                allowed.contains(method),
                                "Allowed methods for {} should contain {:?}",
                                full_path1, method
                            );
                        }
                    }
                    _ => {
                        prop_assert!(false, "DELETE should return MethodNotAllowed for {}", full_path1);
                    }
                }
            }
        }

        /// Property: Three or more routers can be composed without conflicts
        ///
        /// For any set of three routers nested under different prefixes,
        /// all routes should be registered without conflicts.
        #[test]
        fn prop_three_routers_composition(
            prefix1 in "[a-z]{1,3}",
            prefix2 in "[a-z]{4,6}",
            prefix3 in "[a-z]{7,9}",
            num_routes in 1..3usize,
        ) {
            let prefix1 = format!("/{}", prefix1);
            let prefix2 = format!("/{}", prefix2);
            let prefix3 = format!("/{}", prefix3);

            async fn handler() -> &'static str { "handler" }

            // Create three routers with same number of routes
            let mut router1 = Router::new();
            let mut router2 = Router::new();
            let mut router3 = Router::new();

            for i in 0..num_routes {
                let path = format!("/item{}", i);
                router1 = router1.route(&path, get(handler));
                router2 = router2.route(&path, get(handler));
                router3 = router3.route(&path, get(handler));
            }

            // Nest all three routers
            let app = Router::new()
                .nest(&prefix1, router1)
                .nest(&prefix2, router2)
                .nest(&prefix3, router3);

            let routes = app.registered_routes();

            // Property: Total route count should be 3 * num_routes
            let expected_count = 3 * num_routes;
            prop_assert_eq!(
                routes.len(),
                expected_count,
                "Should have {} routes, got {}",
                expected_count, routes.len()
            );

            // Property: All routes should be matchable
            for i in 0..num_routes {
                let path1 = format!("{}/item{}", prefix1, i);
                let path2 = format!("{}/item{}", prefix2, i);
                let path3 = format!("{}/item{}", prefix3, i);

                match app.match_route(&path1, &Method::GET) {
                    RouteMatch::Found { .. } => {}
                    _ => prop_assert!(false, "Route '{}' should be found", path1),
                }
                match app.match_route(&path2, &Method::GET) {
                    RouteMatch::Found { .. } => {}
                    _ => prop_assert!(false, "Route '{}' should be found", path2),
                }
                match app.match_route(&path3, &Method::GET) {
                    RouteMatch::Found { .. } => {}
                    _ => prop_assert!(false, "Route '{}' should be found", path3),
                }
            }
        }
    }
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: Nested routes with same path structure but different param names conflict
        ///
        /// For any existing route with a parameter and a nested route that would create
        /// the same path structure with a different parameter name, the router should
        /// detect and report the conflict.
        #[test]
        fn prop_nested_route_conflict_different_param_names(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
            route_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 0..2),
            param1 in "[a-z][a-z0-9]{1,5}",
            param2 in "[a-z][a-z0-9]{1,5}",
        ) {
            // Ensure param names are different
            prop_assume!(param1 != param2);

            async fn handler1() -> &'static str { "handler1" }
            async fn handler2() -> &'static str { "handler2" }

            let prefix = format!("/{}", prefix_segments.join("/"));

            // Build the existing route path (with param1)
            let existing_path = if route_segments.is_empty() {
                format!("{}/{{{}}}", prefix, param1)
            } else {
                format!("{}/{}/{{{}}}", prefix, route_segments.join("/"), param1)
            };

            // Build the nested route path (with param2)
            let nested_path = if route_segments.is_empty() {
                format!("/{{{}}}", param2)
            } else {
                format!("/{}/{{{}}}", route_segments.join("/"), param2)
            };

            // Try to create a conflict
            let result = catch_unwind(AssertUnwindSafe(|| {
                let parent = Router::new().route(&existing_path, get(handler1));
                let nested = Router::new().route(&nested_path, get(handler2));
                let _app = parent.nest(&prefix, nested);
            }));

            // Property: Should detect conflict
            prop_assert!(
                result.is_err(),
                "Nested route '{}{}' should conflict with existing route '{}' but didn't",
                prefix, nested_path, existing_path
            );

            // Property: Error message should contain conflict information
            if let Err(panic_info) = result {
                if let Some(msg) = panic_info.downcast_ref::<String>() {
                    prop_assert!(
                        msg.contains("ROUTE CONFLICT DETECTED"),
                        "Error should contain 'ROUTE CONFLICT DETECTED', got: {}",
                        msg
                    );
                    prop_assert!(
                        msg.contains("Existing:") && msg.contains("New:"),
                        "Error should contain both 'Existing:' and 'New:' labels, got: {}",
                        msg
                    );
                }
            }
        }

        /// Property: Nested routes with exact same path conflict
        ///
        /// For any existing route and a nested route that would create the exact
        /// same path, the router should detect and report the conflict.
        #[test]
        fn prop_nested_route_conflict_exact_same_path(
            prefix_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
            route_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
        ) {
            async fn handler1() -> &'static str { "handler1" }
            async fn handler2() -> &'static str { "handler2" }

            let prefix = format!("/{}", prefix_segments.join("/"));
            let route_path = format!("/{}", route_segments.join("/"));

            // Build the full existing path
            let existing_path = format!("{}{}", prefix, route_path);

            // Try to create a conflict by nesting a route that creates the same path
            let result = catch_unwind(AssertUnwindSafe(|| {
                let parent = Router::new().route(&existing_path, get(handler1));
                let nested = Router::new().route(&route_path, get(handler2));
                let _app = parent.nest(&prefix, nested);
            }));

            // Property: Should detect conflict
            prop_assert!(
                result.is_err(),
                "Nested route '{}{}' should conflict with existing route '{}' but didn't",
                prefix, route_path, existing_path
            );
        }

        /// Property: Nested routes under different prefixes don't conflict
        ///
        /// For any two nested routers with the same internal routes but different
        /// prefixes, they should not conflict.
        #[test]
        fn prop_nested_routes_different_prefixes_no_conflict(
            prefix1_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
            prefix2_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
            route_segments in prop::collection::vec("[a-z][a-z0-9]{1,5}", 1..3),
            has_param in any::<bool>(),
        ) {
            // Build prefixes
            let prefix1 = format!("/{}", prefix1_segments.join("/"));
            let prefix2 = format!("/{}", prefix2_segments.join("/"));

            // Ensure prefixes are different
            prop_assume!(prefix1 != prefix2);

            async fn handler1() -> &'static str { "handler1" }
            async fn handler2() -> &'static str { "handler2" }

            // Build the route path
            let route_path = if has_param {
                format!("/{}/{{id}}", route_segments.join("/"))
            } else {
                format!("/{}", route_segments.join("/"))
            };

            // Try to nest both routers - should NOT conflict
            let result = catch_unwind(AssertUnwindSafe(|| {
                let nested1 = Router::new().route(&route_path, get(handler1));
                let nested2 = Router::new().route(&route_path, get(handler2));

                let app = Router::new()
                    .nest(&prefix1, nested1)
                    .nest(&prefix2, nested2);

                app.registered_routes().len()
            }));

            // Property: Should NOT conflict
            prop_assert!(
                result.is_ok(),
                "Routes under different prefixes '{}' and '{}' should not conflict",
                prefix1, prefix2
            );

            if let Ok(count) = result {
                prop_assert_eq!(count, 2, "Should have registered 2 routes");
            }
        }

        /// Property: Conflict error message contains resolution guidance
        ///
        /// When a nested route conflict is detected, the error message should
        /// include guidance on how to resolve the conflict.
        #[test]
        fn prop_nested_conflict_error_contains_guidance(
            prefix in "[a-z][a-z0-9]{1,5}",
            segment in "[a-z][a-z0-9]{1,5}",
            param1 in "[a-z][a-z0-9]{1,5}",
            param2 in "[a-z][a-z0-9]{1,5}",
        ) {
            prop_assume!(param1 != param2);

            async fn handler1() -> &'static str { "handler1" }
            async fn handler2() -> &'static str { "handler2" }

            let prefix = format!("/{}", prefix);
            let existing_path = format!("{}/{}/{{{}}}", prefix, segment, param1);
            let nested_path = format!("/{}/{{{}}}", segment, param2);

            let result = catch_unwind(AssertUnwindSafe(|| {
                let parent = Router::new().route(&existing_path, get(handler1));
                let nested = Router::new().route(&nested_path, get(handler2));
                let _app = parent.nest(&prefix, nested);
            }));

            prop_assert!(result.is_err(), "Should have detected conflict");

            if let Err(panic_info) = result {
                if let Some(msg) = panic_info.downcast_ref::<String>() {
                    prop_assert!(
                        msg.contains("How to resolve:"),
                        "Error should contain 'How to resolve:' guidance, got: {}",
                        msg
                    );
                    prop_assert!(
                        msg.contains("Use different path patterns") ||
                        msg.contains("different path patterns"),
                        "Error should suggest using different path patterns, got: {}",
                        msg
                    );
                }
            }
        }
    }
}
