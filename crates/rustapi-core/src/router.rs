//! Router implementation using radix tree (matchit)

use crate::handler::{into_boxed_handler, BoxedHandler, Handler};
use http::{Extensions, Method};
use matchit::Router as MatchitRouter;
use rustapi_openapi::Operation;
use std::collections::HashMap;
use std::sync::Arc;

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
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            inner: MatchitRouter::new(),
            state: Arc::new(Extensions::new()),
        }
    }

    /// Add a route
    pub fn route(mut self, path: &str, method_router: MethodRouter) -> Self {
        // Convert {param} style to :param for matchit
        let matchit_path = convert_path_params(path);
        
        match self.inner.insert(matchit_path.clone(), method_router) {
            Ok(_) => {}
            Err(e) => {
                panic!("Route conflict: {} - {}", path, e);
            }
        }
        self
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
}
