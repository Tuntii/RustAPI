//! Handler trait and utilities

use crate::extract::FromRequest;
use crate::request::Request;
use crate::response::{IntoResponse, Response};
use rustapi_openapi::{Operation, OperationModifier, ResponseModifier};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

/// Trait representing an async handler function
pub trait Handler<T>: Clone + Send + Sync + Sized + 'static {
    /// The response type
    type Future: Future<Output = Response> + Send + 'static;

    /// Call the handler with the request
    fn call(self, req: Request) -> Self::Future;
    
    /// Update the OpenAPI operation
    fn update_operation(op: &mut Operation);
}

/// Wrapper to convert a Handler into a tower Service
pub struct HandlerService<H, T> {
    handler: H,
    _marker: PhantomData<fn() -> T>,
}

impl<H, T> HandlerService<H, T> {
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            _marker: PhantomData,
        }
    }
}

impl<H: Clone, T> Clone for HandlerService<H, T> {
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            _marker: PhantomData,
        }
    }
}

// Implement Handler for async functions with 0-6 extractors

// 0 args
impl<F, Fut, Res> Handler<()> for F
where
    F: FnOnce() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse + ResponseModifier,
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

    fn call(self, _req: Request) -> Self::Future {
        Box::pin(async move {
            self().await.into_response()
        })
    }
    
    fn update_operation(op: &mut Operation) {
        Res::update_response(op);
    }
}

// 1 arg
impl<F, Fut, Res, T1> Handler<(T1,)> for F
where
    F: FnOnce(T1) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse + ResponseModifier,
    T1: FromRequest + OperationModifier + Send + 'static,
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

    fn call(self, mut req: Request) -> Self::Future {
        Box::pin(async move {
            let t1 = match T1::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            self(t1).await.into_response()
        })
    }
    
    fn update_operation(op: &mut Operation) {
        T1::update_operation(op);
        Res::update_response(op);
    }
}

// 2 args
impl<F, Fut, Res, T1, T2> Handler<(T1, T2)> for F
where
    F: FnOnce(T1, T2) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse + ResponseModifier,
    T1: FromRequest + OperationModifier + Send + 'static,
    T2: FromRequest + OperationModifier + Send + 'static,
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

    fn call(self, mut req: Request) -> Self::Future {
        Box::pin(async move {
            let t1 = match T1::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            let t2 = match T2::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            self(t1, t2).await.into_response()
        })
    }
    
    fn update_operation(op: &mut Operation) {
        T1::update_operation(op);
        T2::update_operation(op);
        Res::update_response(op);
    }
}

// 3 args
impl<F, Fut, Res, T1, T2, T3> Handler<(T1, T2, T3)> for F
where
    F: FnOnce(T1, T2, T3) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse + ResponseModifier,
    T1: FromRequest + OperationModifier + Send + 'static,
    T2: FromRequest + OperationModifier + Send + 'static,
    T3: FromRequest + OperationModifier + Send + 'static,
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

    fn call(self, mut req: Request) -> Self::Future {
        Box::pin(async move {
            let t1 = match T1::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            let t2 = match T2::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            let t3 = match T3::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            self(t1, t2, t3).await.into_response()
        })
    }
    
    fn update_operation(op: &mut Operation) {
        T1::update_operation(op);
        T2::update_operation(op);
        T3::update_operation(op);
        Res::update_response(op);
    }
}

// 4 args
impl<F, Fut, Res, T1, T2, T3, T4> Handler<(T1, T2, T3, T4)> for F
where
    F: FnOnce(T1, T2, T3, T4) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse + ResponseModifier,
    T1: FromRequest + OperationModifier + Send + 'static,
    T2: FromRequest + OperationModifier + Send + 'static,
    T3: FromRequest + OperationModifier + Send + 'static,
    T4: FromRequest + OperationModifier + Send + 'static,
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

    fn call(self, mut req: Request) -> Self::Future {
        Box::pin(async move {
            let t1 = match T1::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            let t2 = match T2::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            let t3 = match T3::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            let t4 = match T4::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            self(t1, t2, t3, t4).await.into_response()
        })
    }
    
    fn update_operation(op: &mut Operation) {
        T1::update_operation(op);
        T2::update_operation(op);
        T3::update_operation(op);
        T4::update_operation(op);
        Res::update_response(op);
    }
}

// 5 args
impl<F, Fut, Res, T1, T2, T3, T4, T5> Handler<(T1, T2, T3, T4, T5)> for F
where
    F: FnOnce(T1, T2, T3, T4, T5) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse + ResponseModifier,
    T1: FromRequest + OperationModifier + Send + 'static,
    T2: FromRequest + OperationModifier + Send + 'static,
    T3: FromRequest + OperationModifier + Send + 'static,
    T4: FromRequest + OperationModifier + Send + 'static,
    T5: FromRequest + OperationModifier + Send + 'static,
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

    fn call(self, mut req: Request) -> Self::Future {
        Box::pin(async move {
            let t1 = match T1::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            let t2 = match T2::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            let t3 = match T3::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            let t4 = match T4::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            let t5 = match T5::from_request(&mut req).await {
                Ok(v) => v,
                Err(e) => return e.into_response(),
            };
            self(t1, t2, t3, t4, t5).await.into_response()
        })
    }
    
    fn update_operation(op: &mut Operation) {
        T1::update_operation(op);
        T2::update_operation(op);
        T3::update_operation(op);
        T4::update_operation(op);
        T5::update_operation(op);
        Res::update_response(op);
    }
}

// Type-erased handler for storage in router
pub(crate) type BoxedHandler = Box<
    dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync
>;

/// Create a boxed handler from any Handler
pub(crate) fn into_boxed_handler<H, T>(handler: H) -> BoxedHandler
where
    H: Handler<T>,
    T: 'static,
{
    Box::new(move |req| {
        let handler = handler.clone();
        Box::pin(async move {
            handler.call(req).await
        })
    })
}

/// Trait for handlers with route metadata (generated by #[rustapi::get], etc.)
///
/// This trait provides the path and method information for a handler,
/// allowing `.mount(handler)` to automatically register the route.
pub trait RouteHandler<T>: Handler<T> {
    /// The path pattern for this route (e.g., "/users/{id}")
    const PATH: &'static str;
    /// The HTTP method for this route (e.g., "GET")
    const METHOD: &'static str;
}

/// Represents a route definition that can be registered with .mount()
pub struct Route {
    pub(crate) path: &'static str,
    pub(crate) method: &'static str,
    pub(crate) handler: BoxedHandler,
    pub(crate) operation: Operation,
}

impl Route {
    /// Create a new route from a handler with path and method
    pub fn new<H, T>(path: &'static str, method: &'static str, handler: H) -> Self
    where
        H: Handler<T>,
        T: 'static,
    {
        let mut operation = Operation::new();
        H::update_operation(&mut operation);
        
        Self {
            path,
            method,
            handler: into_boxed_handler(handler),
            operation,
        }
    }
}

/// Helper macro to create a Route from a handler with RouteHandler trait
#[macro_export]
macro_rules! route {
    ($handler:ident) => {{
        $crate::Route::new(
            $handler::PATH,
            $handler::METHOD,
            $handler,
        )
    }};
}

/// Create a GET route
pub fn get_route<H, T>(path: &'static str, handler: H) -> Route
where
    H: Handler<T>,
    T: 'static,
{
    Route::new(path, "GET", handler)
}

/// Create a POST route
pub fn post_route<H, T>(path: &'static str, handler: H) -> Route
where
    H: Handler<T>,
    T: 'static,
{
    Route::new(path, "POST", handler)
}

/// Create a PUT route
pub fn put_route<H, T>(path: &'static str, handler: H) -> Route
where
    H: Handler<T>,
    T: 'static,
{
    Route::new(path, "PUT", handler)
}

/// Create a PATCH route 
pub fn patch_route<H, T>(path: &'static str, handler: H) -> Route
where
    H: Handler<T>,
    T: 'static,
{
    Route::new(path, "PATCH", handler)
}

/// Create a DELETE route
pub fn delete_route<H, T>(path: &'static str, handler: H) -> Route
where
    H: Handler<T>,
    T: 'static,
{
    Route::new(path, "DELETE", handler)
}
