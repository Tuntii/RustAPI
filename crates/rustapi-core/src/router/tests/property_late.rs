use crate::router::{convert_path_params, get, post, put, MethodRouter, RouteMatch, Router};
use http::Method;
use proptest::prelude::*;
use std::panic::{catch_unwind, AssertUnwindSafe};

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
