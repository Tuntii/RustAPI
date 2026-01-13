//! HTTP server implementation

use crate::error::ApiError;
use crate::interceptor::InterceptorChain;
use crate::middleware::{BoxedNext, LayerStack};
use crate::request::Request;
use crate::response::IntoResponse;
use crate::router::{RouteMatch, Router};
use bytes::Bytes;
use http::{header, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
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

    /// Run the server
    pub async fn run(self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr: SocketAddr = addr.parse()?;
        let listener = TcpListener::bind(addr).await?;

        info!("🚀 RustAPI server running on http://{}", addr);

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let router = self.router.clone();
            let layers = self.layers.clone();
            let interceptors = self.interceptors.clone();

            tokio::spawn(async move {
                let service = service_fn(move |req: hyper::Request<Incoming>| {
                    let router = router.clone();
                    let layers = layers.clone();
                    let interceptors = interceptors.clone();
                    async move {
                        let response =
                            handle_request(router, layers, interceptors, req, remote_addr).await;
                        Ok::<_, Infallible>(response)
                    }
                });

                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    error!("Connection error: {}", err);
                }
            });
        }
    }
}

/// Handle a single HTTP request
async fn handle_request(
    router: Arc<Router>,
    layers: Arc<LayerStack>,
    interceptors: Arc<InterceptorChain>,
    req: hyper::Request<Incoming>,
    _remote_addr: SocketAddr,
) -> hyper::Response<Full<Bytes>> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = std::time::Instant::now();

    // Convert hyper request to our Request type first
    let (parts, body) = req.into_parts();

    // Match the route to get path params
    let (handler, params) = match router.match_route(&path, &method) {
        RouteMatch::Found { handler, params } => (handler.clone(), params),
        RouteMatch::NotFound => {
            let response = ApiError::not_found(format!("No route found for {} {}", method, path))
                .into_response();
            log_request(&method, &path, response.status(), start);
            return response;
        }
        RouteMatch::MethodNotAllowed { allowed } => {
            let allowed_str: Vec<&str> = allowed.iter().map(|m| m.as_str()).collect();
            let mut response = ApiError::new(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                format!("Method {} not allowed for {}", method, path),
            )
            .into_response();

            response
                .headers_mut()
                .insert(header::ALLOW, allowed_str.join(", ").parse().unwrap());
            log_request(&method, &path, response.status(), start);
            return response;
        }
    };

    // Build Request (initially streaming)
    let request = Request::new(
        parts,
        crate::request::BodyVariant::Streaming(body),
        router.state_ref(),
        params,
    );

    // Apply request interceptors (in registration order)
    let request = interceptors.intercept_request(request);

    // Create the final handler as a BoxedNext
    let final_handler: BoxedNext = Arc::new(move |req: Request| {
        let handler = handler.clone();
        Box::pin(async move { handler(req).await })
            as std::pin::Pin<
                Box<dyn std::future::Future<Output = crate::response::Response> + Send + 'static>,
            >
    });

    // Execute through middleware stack
    let response = layers.execute(request, final_handler).await;

    // Apply response interceptors (in reverse registration order)
    let response = interceptors.intercept_response(response);

    log_request(&method, &path, response.status(), start);
    response
}

/// Log request completion
fn log_request(method: &http::Method, path: &str, status: StatusCode, start: std::time::Instant) {
    let elapsed = start.elapsed();

    if status.is_success() {
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
