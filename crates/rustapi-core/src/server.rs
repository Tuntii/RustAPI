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
use hyper::service::service_fn;
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

                    let io = TokioIo::new(stream);
                    let router = router.clone();
                    let layers = layers.clone();
                    let interceptors = interceptors.clone();

                    // Spawn connection handler as independent task for better parallelism
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

                        if let Err(err) = http1::Builder::new()
                            .keep_alive(true)
                            .serve_connection(io, service)
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

/// Handle a single HTTP request
async fn handle_request(
    router: Arc<Router>,
    layers: Arc<LayerStack>,
    interceptors: Arc<InterceptorChain>,
    req: hyper::Request<Incoming>,
    _remote_addr: SocketAddr,
) -> hyper::Response<Body> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = std::time::Instant::now();

    // Convert hyper request to our Request type first
    let (parts, body) = req.into_parts();

    // Build Request with empty path params (will be set after route matching)
    let request = Request::new(
        parts,
        crate::request::BodyVariant::Streaming(body),
        router.state_ref(),
        crate::path_params::PathParams::new(),
    );

    // Apply request interceptors (in registration order)
    let request = interceptors.intercept_request(request);

    // Create the routing handler that does route matching inside the middleware chain
    // This allows CORS and other middleware to intercept requests BEFORE route matching
    let router_clone = router.clone();
    let path_clone = path.clone();
    let method_clone = method.clone();
    let routing_handler: BoxedNext = Arc::new(move |mut req: Request| {
        let router = router_clone.clone();
        let path = path_clone.clone();
        let method = method_clone.clone();
        Box::pin(async move {
            match router.match_route(&path, &method) {
                RouteMatch::Found { handler, params } => {
                    // Set path params on the request
                    req.set_path_params(params);
                    handler(req).await
                }
                RouteMatch::NotFound => {
                    ApiError::not_found(format!("No route found for {} {}", method, path))
                        .into_response()
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
                    response
                }
            }
        })
            as std::pin::Pin<
                Box<dyn std::future::Future<Output = crate::response::Response> + Send + 'static>,
            >
    });

    // Execute through middleware stack - middleware runs FIRST, then routing
    let response = layers.execute(request, routing_handler).await;

    // Apply response interceptors (in reverse registration order)
    let response = interceptors.intercept_response(response);

    log_request(&method, &path, response.status(), start);
    response
}

/// Log request completion
#[inline(always)]
fn log_request(method: &http::Method, path: &str, status: StatusCode, start: std::time::Instant) {
    // Skip logging in release builds for performance, unless tracing feature is enabled
    #[cfg(feature = "tracing")]
    {
        let elapsed = start.elapsed();

        // 1xx (Informational), 2xx (Success), 3xx (Redirection) are considered successful requests
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
    
    // Suppress unused variable warnings when tracing is disabled
    #[cfg(not(feature = "tracing"))]
    {
        let _ = (method, path, status, start);
    }
}
