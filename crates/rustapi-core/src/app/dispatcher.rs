use crate::interceptor::InterceptorChain;
use crate::middleware::{BoxedNext, LayerStack};
use crate::router::Router;
use crate::{Request, Response};
use http::Extensions;
use std::sync::Arc;

/// A dispatcher that can drive requests through the RustAPI pipeline
/// (interceptors + layers + router) without any network or serialization overhead.
///
/// Obtained via [`RustApi::request_dispatcher`].
#[derive(Clone)]
pub struct RequestDispatcher {
    pub(super) router: Arc<Router>,
    pub(super) layers: LayerStack,
    pub(super) interceptors: InterceptorChain,
}

impl RequestDispatcher {
    /// Returns the shared state Extensions from the underlying router.
    /// Useful for in-process request construction to preserve `State<T>` etc.
    pub fn state_ref(&self) -> Arc<Extensions> {
        self.router.state_ref()
    }

    /// Dispatch a request through the full stack (interceptors, middleware layers,
    /// route handler, and response interceptors).
    ///
    /// This replicates the logic used by the normal HTTP server.
    pub async fn dispatch(&self, request: Request) -> Response {
        let req = self.interceptors.intercept_request(request);

        let path = req.path().to_owned();
        let method = req.method().clone();

        let response = if self.layers.is_empty() {
            crate::server::route_request_direct(&self.router, req, &path, &method).await
        } else {
            let router = self.router.clone();
            let p = path.clone();
            let m = method.clone();

            let routing_handler: BoxedNext = Arc::new(move |r: Request| {
                let router = router.clone();
                let pp = p.clone();
                let mm = m.clone();
                Box::pin(async move { crate::server::route_request(&router, r, &pp, &mm).await })
            });

            self.layers.execute(req, routing_handler).await
        };

        self.interceptors.intercept_response(response)
    }
}
