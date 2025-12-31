//! Router implementation using radix tree (matchit)
//!
//! This module provides HTTP routing functionality for RustAPI. Routes are
//! registered using path patterns and HTTP method handlers.
//!
//! # Path Patterns
//!
//! Routes support dynamic path parameters using `{param}` syntax:
//!
//! - `/users` - Static path
//! - `/users/{id}` - Single parameter
//! - `/users/{user_id}/posts/{post_id}` - Multiple parameters
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_core::{Router, get, post, put, delete};
//!
//! async fn list_users() -> &'static str { "List users" }
//! async fn get_user() -> &'static str { "Get user" }
//! async fn create_user() -> &'static str { "Create user" }
//! async fn update_user() -> &'static str { "Update user" }
//! async fn delete_user() -> &'static str { "Delete user" }
//!
//! let router = Router::new()
//!     .route("/users", get(list_users).post(create_user))
//!     .route("/users/{id}", get(get_user).put(update_user).delete(delete_user));
//! ```
//!
//! # Method Chaining
//!
//! Multiple HTTP methods can be registered for the same path using method chaining:
//!
//! ```rust,ignore
//! .route("/users", get(list).post(create))
//! .route("/users/{id}", get(show).put(update).delete(destroy))
//! ```
//!
//! # Route Conflict Detection
//!
//! The router detects conflicting routes at registration time and provides
//! helpful error messages with resolution guidance.

use crate::handler::{into_boxed_handler, BoxedHandler, Handler};
use http::{Extensions, Method};
use matchit::Router as MatchitRouter;
use rustapi_openapi::Operation;
use std::collections::HashMap;
use std::sync::Arc;

/// Information about a registered route for conflict detection
#[derive(Debug, Clone)]
pub struct RouteInfo {
    /// The original path pattern (e.g., "/users/{id}")
    pub path: String,
    /// The HTTP methods registered for this path
    pub methods: Vec<Method>,
}

/// Error returned when a route conflict is detected
#[derive(Debug, Clone)]
pub struct RouteConflictError {
    /// The path that was being registered
    pub new_path: String,
    /// The HTTP method that conflicts
    pub method: Option<Method>,
    /// The existing path that conflicts
    pub existing_path: String,
    /// Detailed error message from the underlying router
    pub details: String,
}

impl std::fmt::Display for RouteConflictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n╭─────────────────────────────────────────────────────────────╮")?;
        writeln!(f, "│                    ROUTE CONFLICT DETECTED                   │")?;
        writeln!(f, "╰─────────────────────────────────────────────────────────────╯")?;
        writeln!(f)?;
        writeln!(f, "  Conflicting routes:")?;
        writeln!(f, "    → Existing: {}", self.existing_path)?;
        writeln!(f, "    → New:      {}", self.new_path)?;
        writeln!(f)?;
        if let Some(ref method) = self.method {
            writeln!(f, "  HTTP Method: {}", method)?;
            writeln!(f)?;
        }
        writeln!(f, "  Details: {}", self.details)?;
        writeln!(f)?;
        writeln!(f, "  How to resolve:")?;
        writeln!(f, "    1. Use different path patterns for each route")?;
        writeln!(f, "    2. If paths must be similar, ensure parameter names differ")?;
        writeln!(f, "    3. Consider using different HTTP methods if appropriate")?;
        writeln!(f)?;
        writeln!(f, "  Example:")?;
        writeln!(f, "    Instead of:")?;
        writeln!(f, "      .route(\"/users/{{id}}\", get(handler1))")?;
        writeln!(f, "      .route(\"/users/{{user_id}}\", get(handler2))")?;
        writeln!(f)?;
        writeln!(f, "    Use:")?;
        writeln!(f, "      .route(\"/users/{{id}}\", get(handler1))")?;
        writeln!(f, "      .route(\"/users/{{id}}/profile\", get(handler2))")?;
        Ok(())
    }
}

impl std::error::Error for RouteConflictError {}

/// HTTP method router for a single path
pub struct MethodRouter {
    handlers: HashMap<Method, BoxedHandler>,
    pub(crate) operations: HashMap<Method, Operation>,
}

impl MethodRouter {
    /// Create a new empty method router
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            operations: HashMap::new(),
        }
    }

    /// Add a handler for a specific method
    fn on(mut self, method: Method, handler: BoxedHandler, operation: Operation) -> Self {
        self.handlers.insert(method.clone(), handler);
        self.operations.insert(method, operation);
        self
    }

    /// Get handler for a method
    pub(crate) fn get_handler(&self, method: &Method) -> Option<&BoxedHandler> {
        self.handlers.get(method)
    }

    /// Get allowed methods for 405 response
    pub(crate) fn allowed_methods(&self) -> Vec<Method> {
        self.handlers.keys().cloned().collect()
    }

    /// Create from pre-boxed handlers (internal use)
    pub(crate) fn from_boxed(handlers: HashMap<Method, BoxedHandler>) -> Self {
        Self { 
            handlers,
            operations: HashMap::new(), // Operations lost when using raw boxed handlers for now
        }
    }
}

impl Default for MethodRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a GET route handler
pub fn get<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: 'static,
{
    let mut op = Operation::new();
    H::update_operation(&mut op);
    MethodRouter::new().on(Method::GET, into_boxed_handler(handler), op)
}

/// Create a POST route handler
pub fn post<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: 'static,
{
    let mut op = Operation::new();
    H::update_operation(&mut op);
    MethodRouter::new().on(Method::POST, into_boxed_handler(handler), op)
}

/// Create a PUT route handler
pub fn put<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: 'static,
{
    let mut op = Operation::new();
    H::update_operation(&mut op);
    MethodRouter::new().on(Method::PUT, into_boxed_handler(handler), op)
}

/// Create a PATCH route handler
pub fn patch<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: 'static,
{
    let mut op = Operation::new();
    H::update_operation(&mut op);
    MethodRouter::new().on(Method::PATCH, into_boxed_handler(handler), op)
}

/// Create a DELETE route handler
pub fn delete<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: 'static,
{
    let mut op = Operation::new();
    H::update_operation(&mut op);
    MethodRouter::new().on(Method::DELETE, into_boxed_handler(handler), op)
}

/// Main router
pub struct Router {
    inner: MatchitRouter<MethodRouter>,
    state: Arc<Extensions>,
    /// Track registered routes for conflict detection
    registered_routes: HashMap<String, RouteInfo>,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            inner: MatchitRouter::new(),
            state: Arc::new(Extensions::new()),
            registered_routes: HashMap::new(),
        }
    }

    /// Add a route
    pub fn route(mut self, path: &str, method_router: MethodRouter) -> Self {
        // Convert {param} style to :param for matchit
        let matchit_path = convert_path_params(path);
        
        // Get the methods being registered
        let methods: Vec<Method> = method_router.handlers.keys().cloned().collect();
        
        match self.inner.insert(matchit_path.clone(), method_router) {
            Ok(_) => {
                // Track the registered route
                self.registered_routes.insert(
                    matchit_path.clone(),
                    RouteInfo {
                        path: path.to_string(),
                        methods,
                    },
                );
            }
            Err(e) => {
                // Find the existing conflicting route
                let existing_path = self.find_conflicting_route(&matchit_path)
                    .map(|info| info.path.clone())
                    .unwrap_or_else(|| "<unknown>".to_string());
                
                let conflict_error = RouteConflictError {
                    new_path: path.to_string(),
                    method: methods.first().cloned(),
                    existing_path,
                    details: e.to_string(),
                };
                
                panic!("{}", conflict_error);
            }
        }
        self
    }
    
    /// Find a conflicting route by checking registered routes
    fn find_conflicting_route(&self, matchit_path: &str) -> Option<&RouteInfo> {
        // Try to find an exact match first
        if let Some(info) = self.registered_routes.get(matchit_path) {
            return Some(info);
        }
        
        // Try to find a route that would conflict (same structure but different param names)
        let normalized_new = normalize_path_for_comparison(matchit_path);
        
        for (registered_path, info) in &self.registered_routes {
            let normalized_existing = normalize_path_for_comparison(registered_path);
            if normalized_new == normalized_existing {
                return Some(info);
            }
        }
        
        None
    }

    /// Add application state
    pub fn state<S: Clone + Send + Sync + 'static>(mut self, state: S) -> Self {
        let extensions = Arc::make_mut(&mut self.state);
        extensions.insert(state);
        self
    }

    /// Nest another router under a prefix
    pub fn nest(self, _prefix: &str, _router: Router) -> Self {
        // TODO: Implement router nesting
        self
    }

    /// Match a request and return the handler + params
    pub(crate) fn match_route(
        &self,
        path: &str,
        method: &Method,
    ) -> RouteMatch<'_> {
        match self.inner.at(path) {
            Ok(matched) => {
                let method_router = matched.value;
                
                if let Some(handler) = method_router.get_handler(method) {
                    // Convert params to HashMap
                    let params: HashMap<String, String> = matched
                        .params
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect();
                    
                    RouteMatch::Found { handler, params }
                } else {
                    RouteMatch::MethodNotAllowed {
                        allowed: method_router.allowed_methods(),
                    }
                }
            }
            Err(_) => RouteMatch::NotFound,
        }
    }

    /// Get shared state
    pub(crate) fn state_ref(&self) -> Arc<Extensions> {
        self.state.clone()
    }
    
    /// Get registered routes (for testing and debugging)
    pub fn registered_routes(&self) -> &HashMap<String, RouteInfo> {
        &self.registered_routes
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of route matching
pub(crate) enum RouteMatch<'a> {
    Found {
        handler: &'a BoxedHandler,
        params: HashMap<String, String>,
    },
    NotFound,
    MethodNotAllowed {
        allowed: Vec<Method>,
    },
}

/// Convert {param} style to :param for matchit
fn convert_path_params(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    
    for ch in path.chars() {
        match ch {
            '{' => {
                result.push(':');
            }
            '}' => {
                // Skip closing brace
            }
            _ => {
                result.push(ch);
            }
        }
    }
    
    result
}

/// Normalize a path for conflict comparison by replacing parameter names with a placeholder
fn normalize_path_for_comparison(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut in_param = false;
    
    for ch in path.chars() {
        match ch {
            ':' => {
                in_param = true;
                result.push_str(":_");
            }
            '/' => {
                in_param = false;
                result.push('/');
            }
            _ if in_param => {
                // Skip parameter name characters
            }
            _ => {
                result.push(ch);
            }
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(normalize_path_for_comparison("/users/:user_id"), "/users/:_");
        assert_eq!(
            normalize_path_for_comparison("/users/:id/posts/:post_id"),
            "/users/:_/posts/:_"
        );
        assert_eq!(normalize_path_for_comparison("/static/path"), "/static/path");
    }
    
    #[test]
    #[should_panic(expected = "ROUTE CONFLICT DETECTED")]
    fn test_route_conflict_detection() {
        async fn handler1() -> &'static str { "handler1" }
        async fn handler2() -> &'static str { "handler2" }
        
        let _router = Router::new()
            .route("/users/{id}", get(handler1))
            .route("/users/{user_id}", get(handler2)); // This should panic
    }
    
    #[test]
    fn test_no_conflict_different_paths() {
        async fn handler1() -> &'static str { "handler1" }
        async fn handler2() -> &'static str { "handler2" }
        
        let router = Router::new()
            .route("/users/{id}", get(handler1))
            .route("/users/{id}/profile", get(handler2));
        
        assert_eq!(router.registered_routes().len(), 2);
    }
    
    #[test]
    fn test_route_info_tracking() {
        async fn handler() -> &'static str { "handler" }
        
        let router = Router::new()
            .route("/users/{id}", get(handler));
        
        let routes = router.registered_routes();
        assert_eq!(routes.len(), 1);
        
        let info = routes.get("/users/:id").unwrap();
        assert_eq!(info.path, "/users/{id}");
        assert_eq!(info.methods.len(), 1);
        assert_eq!(info.methods[0], Method::GET);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

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
}
