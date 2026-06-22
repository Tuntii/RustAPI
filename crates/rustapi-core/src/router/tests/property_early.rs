use crate::router::{
    convert_path_params, delete, get, normalize_path_for_comparison, normalize_prefix, patch, post,
    put, MethodRouter, RouteMatch, Router,
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
