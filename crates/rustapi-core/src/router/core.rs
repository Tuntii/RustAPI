use super::conflict::{RouteConflictError, RouteInfo};
use super::match_::{
    convert_path_params, normalize_path_for_comparison, normalize_prefix, RouteMatch,
};
use super::method_router::MethodRouter;
use crate::path_params::PathParams;
use crate::typed_path::TypedPath;
use http::{Extensions, Method};
use matchit::Router as MatchitRouter;
use std::collections::HashMap;
use std::sync::Arc;

/// Main router
#[derive(Clone)]
pub struct Router {
    inner: MatchitRouter<MethodRouter>,
    pub(super) state: Arc<Extensions>,
    /// Track registered routes for conflict detection
    registered_routes: HashMap<String, RouteInfo>,
    /// Store MethodRouters for nesting support (keyed by matchit path)
    method_routers: HashMap<String, MethodRouter>,
    /// Track state type IDs for merging (type name -> whether it's set)
    /// This is a workaround since Extensions doesn't support iteration
    state_type_ids: Vec<std::any::TypeId>,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            inner: MatchitRouter::new(),
            state: Arc::new(Extensions::new()),
            registered_routes: HashMap::new(),
            method_routers: HashMap::new(),
            state_type_ids: Vec::new(),
        }
    }

    /// Add a typed route using a TypedPath
    pub fn typed<P: TypedPath>(self, method_router: MethodRouter) -> Self {
        self.route(P::PATH, method_router)
    }

    /// Add a route
    pub fn route(mut self, path: &str, method_router: MethodRouter) -> Self {
        // Convert {param} style to :param for matchit
        let matchit_path = convert_path_params(path);

        // Get the methods being registered
        let methods: Vec<Method> = method_router.handlers.keys().cloned().collect();

        // Store a clone of the MethodRouter for nesting support
        self.method_routers
            .insert(matchit_path.clone(), method_router.clone());

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
                // Remove the method_router we just added since registration failed
                self.method_routers.remove(&matchit_path);

                // Find the existing conflicting route
                let existing_path = self
                    .find_conflicting_route(&matchit_path)
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
        let type_id = std::any::TypeId::of::<S>();
        let extensions = Arc::make_mut(&mut self.state);
        extensions.insert(state);
        if !self.state_type_ids.contains(&type_id) {
            self.state_type_ids.push(type_id);
        }
        self
    }

    /// Check if state of a given type exists
    pub fn has_state<S: 'static>(&self) -> bool {
        self.state_type_ids.contains(&std::any::TypeId::of::<S>())
    }

    /// Get state type IDs (for testing and debugging)
    pub fn state_type_ids(&self) -> &[std::any::TypeId] {
        &self.state_type_ids
    }

    /// Nest another router under a prefix
    ///
    /// All routes from the nested router will be registered with the prefix
    /// prepended to their paths. State from the nested router is merged into
    /// the parent router (parent state takes precedence for type conflicts).
    ///
    /// # State Merging
    ///
    /// When nesting routers with state:
    /// - If the parent router has state of type T, it is preserved (parent wins)
    /// - If only the nested router has state of type T, it is added to the parent
    /// - State type tracking is merged to enable proper conflict detection
    ///
    /// Note: Due to limitations of `http::Extensions`, automatic state merging
    /// requires using the `merge_state` method for specific types.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rustapi_core::{Router, get};
    ///
    /// async fn list_users() -> &'static str { "List users" }
    /// async fn get_user() -> &'static str { "Get user" }
    ///
    /// let users_router = Router::new()
    ///     .route("/", get(list_users))
    ///     .route("/{id}", get(get_user));
    ///
    /// let app = Router::new()
    ///     .nest("/api/users", users_router);
    ///
    /// // Routes are now:
    /// // GET /api/users/
    /// // GET /api/users/{id}
    /// ```
    ///
    /// # Nesting with State
    ///
    /// The `nest` method automatically tracks state types from the nested router to prevent
    /// conflicts, but it does NOT automatically merge the state values instance by instance.
    /// You should distinctively add state to the parent, or use `merge_state` if you want
    /// to pull a specific state object from the child.
    ///
    /// ```rust,ignore
    /// use rustapi_core::Router;
    /// use std::sync::Arc;
    ///
    /// #[derive(Clone)]
    /// struct Database { /* ... */ }
    ///
    /// let db = Database { /* ... */ };
    ///
    /// // Option 1: Add state to the parent (Recommended)
    /// let api = Router::new()
    ///     .nest("/v1", Router::new()
    ///         .route("/users", get(list_users))) // Needs Database
    ///     .state(db);
    ///
    /// // Option 2: Define specific state in sub-router and merge explicitly
    /// let sub_router = Router::new()
    ///     .state(Database { /* ... */ })
    ///     .route("/items", get(list_items));
    ///
    /// let app = Router::new()
    ///     .merge_state::<Database>(&sub_router) // Pulls Database from sub_router
    ///     .nest("/api", sub_router);
    /// ```
    pub fn nest(mut self, prefix: &str, router: Router) -> Self {
        // 1. Normalize the prefix
        let normalized_prefix = normalize_prefix(prefix);

        // 2. Merge state type IDs from nested router
        // Parent state takes precedence - we only track types, actual values
        // are handled by merge_state calls or by the user adding state to parent
        for type_id in &router.state_type_ids {
            if !self.state_type_ids.contains(type_id) {
                self.state_type_ids.push(*type_id);
            }
        }

        // 3. Collect routes from the nested router before consuming it
        // We need to iterate over registered_routes and get the corresponding MethodRouters
        let nested_routes: Vec<(String, RouteInfo, MethodRouter)> = router
            .registered_routes
            .into_iter()
            .filter_map(|(matchit_path, route_info)| {
                router
                    .method_routers
                    .get(&matchit_path)
                    .map(|mr| (matchit_path, route_info, mr.clone()))
            })
            .collect();

        // 4. Register each nested route with the prefix
        for (matchit_path, route_info, method_router) in nested_routes {
            // Build the prefixed path
            // The matchit_path already has the :param format
            // The route_info.path has the {param} format
            let prefixed_matchit_path = if matchit_path == "/" {
                normalized_prefix.clone()
            } else {
                format!("{}{}", normalized_prefix, matchit_path)
            };

            let prefixed_display_path = if route_info.path == "/" {
                normalized_prefix.clone()
            } else {
                format!("{}{}", normalized_prefix, route_info.path)
            };

            // Store the MethodRouter for future nesting
            self.method_routers
                .insert(prefixed_matchit_path.clone(), method_router.clone());

            // Try to insert into the matchit router
            match self
                .inner
                .insert(prefixed_matchit_path.clone(), method_router)
            {
                Ok(_) => {
                    // Track the registered route
                    self.registered_routes.insert(
                        prefixed_matchit_path,
                        RouteInfo {
                            path: prefixed_display_path,
                            methods: route_info.methods,
                        },
                    );
                }
                Err(e) => {
                    // Remove the method_router we just added since registration failed
                    self.method_routers.remove(&prefixed_matchit_path);

                    // Find the existing conflicting route
                    let existing_path = self
                        .find_conflicting_route(&prefixed_matchit_path)
                        .map(|info| info.path.clone())
                        .unwrap_or_else(|| "<unknown>".to_string());

                    let conflict_error = RouteConflictError {
                        new_path: prefixed_display_path,
                        method: route_info.methods.first().cloned(),
                        existing_path,
                        details: e.to_string(),
                    };

                    panic!("{}", conflict_error);
                }
            }
        }

        self
    }

    /// Merge state from another router into this one
    ///
    /// This method allows explicit state merging when nesting routers.
    /// Parent state takes precedence - if the parent already has state of type S,
    /// the nested state is ignored.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[derive(Clone)]
    /// struct DbPool(String);
    ///
    /// let nested = Router::new().state(DbPool("nested".to_string()));
    /// let parent = Router::new()
    ///     .merge_state::<DbPool>(&nested); // Adds DbPool from nested
    /// ```
    pub fn merge_state<S: Clone + Send + Sync + 'static>(mut self, other: &Router) -> Self {
        let type_id = std::any::TypeId::of::<S>();

        // Parent wins - only merge if parent doesn't have this state type
        if !self.state_type_ids.contains(&type_id) {
            // Try to get the state from the other router
            if let Some(state) = other.state.get::<S>() {
                let extensions = Arc::make_mut(&mut self.state);
                extensions.insert(state.clone());
                self.state_type_ids.push(type_id);
            }
        }

        self
    }

    /// Match a request and return the handler + params
    pub fn match_route(&self, path: &str, method: &Method) -> RouteMatch<'_> {
        match self.inner.at(path) {
            Ok(matched) => {
                let method_router = matched.value;

                if let Some(handler) = method_router.get_handler(method) {
                    // Use stack-optimized PathParams (avoids heap allocation for â‰¤4 params)
                    let params: PathParams = matched
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
    pub fn state_ref(&self) -> Arc<Extensions> {
        self.state.clone()
    }

    /// Get registered routes (for testing and debugging)
    pub fn registered_routes(&self) -> &HashMap<String, RouteInfo> {
        &self.registered_routes
    }

    /// Get method routers (for OpenAPI integration during nesting)
    pub fn method_routers(&self) -> &HashMap<String, MethodRouter> {
        &self.method_routers
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
