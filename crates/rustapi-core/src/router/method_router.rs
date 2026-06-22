use crate::handler::{into_boxed_handler, BoxedHandler, Handler};
use http::Method;
use rustapi_openapi::Operation;
use std::collections::HashMap;

/// HTTP method router for a single path
pub struct MethodRouter {
    pub(super) handlers: HashMap<Method, BoxedHandler>,
    pub(crate) operations: HashMap<Method, Operation>,
    pub(crate) component_registrars: Vec<fn(&mut rustapi_openapi::OpenApiSpec)>,
}

impl Clone for MethodRouter {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
            operations: self.operations.clone(),
            component_registrars: self.component_registrars.clone(),
        }
    }
}

impl MethodRouter {
    /// Create a new empty method router
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            operations: HashMap::new(),
            component_registrars: Vec::new(),
        }
    }

    /// Add a handler for a specific method
    fn on(
        mut self,
        method: Method,
        handler: BoxedHandler,
        operation: Operation,
        component_registrar: fn(&mut rustapi_openapi::OpenApiSpec),
    ) -> Self {
        self.handlers.insert(method.clone(), handler);
        self.operations.insert(method, operation);
        self.component_registrars.push(component_registrar);
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
            component_registrars: Vec::new(),
        }
    }

    /// Insert a pre-boxed handler and its OpenAPI operation (internal use).
    ///
    /// Panics if the same method is inserted twice for the same path.
    pub(crate) fn insert_boxed_with_operation(
        &mut self,
        method: Method,
        handler: BoxedHandler,
        operation: Operation,
        component_registrar: fn(&mut rustapi_openapi::OpenApiSpec),
    ) {
        if self.handlers.contains_key(&method) {
            panic!(
                "Duplicate handler for method {} on the same path",
                method.as_str()
            );
        }

        self.handlers.insert(method.clone(), handler);
        self.operations.insert(method, operation);
        self.component_registrars.push(component_registrar);
    }

    /// Add a GET handler
    pub fn get<H, T>(self, handler: H) -> Self
    where
        H: Handler<T>,
        T: 'static,
    {
        let mut op = Operation::new();
        H::update_operation(&mut op);
        self.on(
            Method::GET,
            into_boxed_handler(handler),
            op,
            <H as Handler<T>>::register_components,
        )
    }

    /// Add a POST handler
    pub fn post<H, T>(self, handler: H) -> Self
    where
        H: Handler<T>,
        T: 'static,
    {
        let mut op = Operation::new();
        H::update_operation(&mut op);
        self.on(
            Method::POST,
            into_boxed_handler(handler),
            op,
            <H as Handler<T>>::register_components,
        )
    }

    /// Add a PUT handler
    pub fn put<H, T>(self, handler: H) -> Self
    where
        H: Handler<T>,
        T: 'static,
    {
        let mut op = Operation::new();
        H::update_operation(&mut op);
        self.on(
            Method::PUT,
            into_boxed_handler(handler),
            op,
            <H as Handler<T>>::register_components,
        )
    }

    /// Add a PATCH handler
    pub fn patch<H, T>(self, handler: H) -> Self
    where
        H: Handler<T>,
        T: 'static,
    {
        let mut op = Operation::new();
        H::update_operation(&mut op);
        self.on(
            Method::PATCH,
            into_boxed_handler(handler),
            op,
            <H as Handler<T>>::register_components,
        )
    }

    /// Add a DELETE handler
    pub fn delete<H, T>(self, handler: H) -> Self
    where
        H: Handler<T>,
        T: 'static,
    {
        let mut op = Operation::new();
        H::update_operation(&mut op);
        self.on(
            Method::DELETE,
            into_boxed_handler(handler),
            op,
            <H as Handler<T>>::register_components,
        )
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
    MethodRouter::new().on(
        Method::GET,
        into_boxed_handler(handler),
        op,
        <H as Handler<T>>::register_components,
    )
}

/// Create a POST route handler
pub fn post<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: 'static,
{
    let mut op = Operation::new();
    H::update_operation(&mut op);
    MethodRouter::new().on(
        Method::POST,
        into_boxed_handler(handler),
        op,
        <H as Handler<T>>::register_components,
    )
}

/// Create a PUT route handler
pub fn put<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: 'static,
{
    let mut op = Operation::new();
    H::update_operation(&mut op);
    MethodRouter::new().on(
        Method::PUT,
        into_boxed_handler(handler),
        op,
        <H as Handler<T>>::register_components,
    )
}

/// Create a PATCH route handler
pub fn patch<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: 'static,
{
    let mut op = Operation::new();
    H::update_operation(&mut op);
    MethodRouter::new().on(
        Method::PATCH,
        into_boxed_handler(handler),
        op,
        <H as Handler<T>>::register_components,
    )
}

/// Create a DELETE route handler
pub fn delete<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: 'static,
{
    let mut op = Operation::new();
    H::update_operation(&mut op);
    MethodRouter::new().on(
        Method::DELETE,
        into_boxed_handler(handler),
        op,
        <H as Handler<T>>::register_components,
    )
}
