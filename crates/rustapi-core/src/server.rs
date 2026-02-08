//! HTTP server implementation

use crate::error::ApiError;
use crate::interceptor::InterceptorChain;
use crate::middleware::{BoxedNext, LayerStack};
use crate::request::Request;
use crate::response::{Body, IntoResponse};
use crate::router::{RouteMatch, Router};

use http::{header, StatusCode};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Internal server struct
pub(crate) struct Server {
    router: Arc<Router>,
    layers: Arc<LayerStack>,
    interceptors: Arc<InterceptorChain>,
}

impl Server {
    pub fn new(router: Router, layers: LayerStack, interceptors: InterceptorChain) -> Self {
        Self {
            router: Arc::new(router),
            layers: Arc::new(layers),
            interceptors: Arc::new(interceptors),
        }
    }

    #[cfg(feature = "http3")]
    pub fn from_shared(
        router: Arc<Router>,
        layers: Arc<LayerStack>,
        interceptors: Arc<InterceptorChain>,
    ) -> Self {
        Self {
            router,
            layers,
            interceptors,
        }
    }

    /// Run the server
    pub async fn run(self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.run_with_shutdown(addr, std::future::pending()).await
    }

    /// Run the server with graceful shutdown signal
    pub async fn run_with_shutdown<F>(
        self,
        addr: &str,
        signal: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let addr: SocketAddr = addr.parse()?;
        let listener = TcpListener::bind(addr).await?;

        info!("🚀 RustAPI server running on http://{}", addr);

        // Arc-wrap self for sharing across tasks
        let router = self.router;
        let layers = self.layers;
        let interceptors = self.interceptors;

        tokio::pin!(signal);

        loop {
            tokio::select! {
                biased; // Prioritize accept over shutdown for better throughput

                accept_result = listener.accept() => {
                    let (stream, remote_addr) = match accept_result {
                        Ok(v) => v,
                        Err(e) => {
                            error!("Accept error: {}", e);
                            continue;
                        }
                    };

                    // Disable Nagle's algorithm for lower latency
                    let _ = stream.set_nodelay(true);

                    let io = TokioIo::new(stream);

                    // Create connection service once - no cloning per request!
                    let conn_service = ConnectionService {
                        router: router.clone(),
                        layers: layers.clone(),
                        interceptors: interceptors.clone(),
                        remote_addr,
                    };

                    // Spawn connection handler as independent task
                    tokio::spawn(async move {
                        if let Err(err) = http1::Builder::new()
                            .keep_alive(true)
                            .pipeline_flush(true) // Flush pipelined responses immediately
                            .serve_connection(io, conn_service)
                            .with_upgrades()
                            .await
                        {
                            // Only log actual errors, not client disconnects
                            if !err.is_incomplete_message() {
                                error!("Connection error: {}", err);
                            }
                        }
                    });
                }
                _ = &mut signal => {
                    info!("Shutdown signal received");
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Connection-level service - avoids Arc cloning per request
#[derive(Clone)]
struct ConnectionService {
    router: Arc<Router>,
    layers: Arc<LayerStack>,
    interceptors: Arc<InterceptorChain>,
    remote_addr: SocketAddr,
}

impl hyper::service::Service<hyper::Request<Incoming>> for ConnectionService {
    type Response = hyper::Response<Body>;
    type Error = Infallible;
    type Future = HandleRequestFuture;

    #[inline(always)]
    fn call(&self, req: hyper::Request<Incoming>) -> Self::Future {
        HandleRequestFuture {
            router: self.router.clone(),
            layers: self.layers.clone(),
            interceptors: self.interceptors.clone(),
            remote_addr: self.remote_addr,
            request: Some(req),
            state: FutureState::Initial,
        }
    }
}

/// Custom future to avoid Box::pin allocation per request
pub struct HandleRequestFuture {
    router: Arc<Router>,
    layers: Arc<LayerStack>,
    interceptors: Arc<InterceptorChain>,
    remote_addr: SocketAddr,
    request: Option<hyper::Request<Incoming>>,
    state: FutureState,
}

enum FutureState {
    Initial,
    Processing(std::pin::Pin<Box<dyn Future<Output = hyper::Response<Body>> + Send>>),
}

impl Future for HandleRequestFuture {
    type Output = Result<hyper::Response<Body>, Infallible>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        loop {
            match &mut self.state {
                FutureState::Initial => {
                    let req = self.request.take().unwrap();
                    let router = self.router.clone();
                    let layers = self.layers.clone();
                    let interceptors = self.interceptors.clone();
                    let remote_addr = self.remote_addr;

                    let fut = Box::pin(handle_request(
                        router,
                        layers,
                        interceptors,
                        req,
                        remote_addr,
                    ));
                    self.state = FutureState::Processing(fut);
                }
                FutureState::Processing(fut) => {
                    return fut.as_mut().poll(cx).map(Ok);
                }
            }
        }
    }
}

/// Handle a single HTTP request
#[inline]
async fn handle_request(
    router: Arc<Router>,
    layers: Arc<LayerStack>,
    interceptors: Arc<InterceptorChain>,
    req: hyper::Request<Incoming>,
    _remote_addr: SocketAddr,
) -> hyper::Response<Body> {
    // Extract method and path before consuming request
    // Clone method (cheap - just an enum) and path to owned string only when needed
    let method = req.method().clone();
    let path = req.uri().path().to_owned();

    // Only measure time when tracing is enabled
    #[cfg(feature = "tracing")]
    let start = std::time::Instant::now();

    // Convert hyper request to our Request type
    let (parts, body) = req.into_parts();

    // Build Request with empty path params (will be set after route matching)
    let request = Request::new(
        parts,
        crate::request::BodyVariant::Streaming(body),
        router.state_ref(),
        crate::path_params::PathParams::new(),
    );

    // ULTRA FAST PATH: No middleware AND no interceptors
    let response = if layers.is_empty() && interceptors.is_empty() {
        route_request_direct(&router, request, &path, &method).await
    } else if layers.is_empty() {
        // Fast path: No middleware, but has interceptors
        let request = interceptors.intercept_request(request);
        let response = route_request_direct(&router, request, &path, &method).await;
        interceptors.intercept_response(response)
    } else {
        // Slow path: Has middleware
        let request = interceptors.intercept_request(request);
        let router_clone = router.clone();
        let path_clone = path.clone();
        let method_clone = method.clone();

        let routing_handler: BoxedNext = Arc::new(move |req: Request| {
            let router = router_clone.clone();
            let path = path_clone.clone();
            let method = method_clone.clone();
            Box::pin(async move { route_request(&router, req, &path, &method).await })
                as std::pin::Pin<
                    Box<
                        dyn std::future::Future<Output = crate::response::Response>
                            + Send
                            + 'static,
                    >,
                >
        });

        let response = layers.execute(request, routing_handler).await;
        interceptors.intercept_response(response)
    };

    #[cfg(feature = "tracing")]
    log_request(&method, &path, response.status(), start);

    response
}

/// Direct routing without middleware chain - maximum performance path
#[inline]
async fn route_request_direct(
    router: &Router,
    mut request: Request,
    path: &str,
    method: &http::Method,
) -> hyper::Response<Body> {
    match router.match_route(path, method) {
        RouteMatch::Found { handler, params } => {
            request.set_path_params(params);
            handler(request).await
        }
        RouteMatch::NotFound => ApiError::not_found("Not found").into_response(),
        RouteMatch::MethodNotAllowed { allowed } => {
            let allowed_str: Vec<&str> = allowed.iter().map(|m| m.as_str()).collect();
            let mut response = ApiError::new(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "Method not allowed",
            )
            .into_response();
            response
                .headers_mut()
                .insert(header::ALLOW, allowed_str.join(", ").parse().unwrap());
            response
        }
    }
}

/// Route request through the router (used by middleware chain)
#[inline]
async fn route_request(
    router: &Router,
    mut request: Request,
    path: &str,
    method: &http::Method,
) -> hyper::Response<Body> {
    match router.match_route(path, method) {
        RouteMatch::Found { handler, params } => {
            request.set_path_params(params);
            handler(request).await
        }
        RouteMatch::NotFound => ApiError::not_found("Not found").into_response(),
        RouteMatch::MethodNotAllowed { allowed } => {
            let allowed_str: Vec<&str> = allowed.iter().map(|m| m.as_str()).collect();
            let mut response = ApiError::new(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "Method not allowed",
            )
            .into_response();
            response
                .headers_mut()
                .insert(header::ALLOW, allowed_str.join(", ").parse().unwrap());
            response
        }
    }
}

/// Log request completion - only compiled when tracing is enabled
#[cfg(feature = "tracing")]
#[inline(always)]
fn log_request(method: &http::Method, path: &str, status: StatusCode, start: std::time::Instant) {
    let elapsed = start.elapsed();

    if status.is_success() || status.is_redirection() || status.is_informational() {
        info!(
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %elapsed.as_millis(),
            "Request completed"
        );
    } else {
        error!(
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %elapsed.as_millis(),
            "Request failed"
        );
    }
}
