//! HTTP/3 server implementation using Quinn + h3
//!
//! This module provides HTTP/3 (QUIC) support for RustAPI.
//! HTTP/3 requires TLS certificates and runs over UDP.
//!
//! # Example
//!
//! ```rust,no_run
//! use rustapi_core::RustApi;
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = RustApi::new()
//!         .route("/", rustapi_core::get(|| async { "Hello HTTP/3!" }))
//!         .with_http3("cert.pem", "key.pem");
//!     
//!     app.run_dual_stack("0.0.0.0:8080").await.unwrap();
//! }
//! ```

use crate::error::ApiError;
use crate::interceptor::InterceptorChain;
use crate::middleware::{BoxedNext, LayerStack};
use crate::request::Request;
use crate::response::IntoResponse;
use crate::router::{RouteMatch, Router};
use bytes::{Buf, Bytes};
use h3::server::RequestStream;
use h3_quinn::BidiStream;
use http::{header, StatusCode};
use quinn::{Endpoint, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};

/// HTTP/3 server configuration
#[derive(Clone)]
pub struct Http3Config {
    /// Path to TLS certificate file (PEM format)
    pub cert_path: String,
    /// Path to private key file (PEM format)
    pub key_path: String,
    /// Port for HTTP/3 server (default: 443)
    pub port: u16,
    /// Bind address (default: "0.0.0.0")
    pub bind_addr: String,
}

impl Default for Http3Config {
    fn default() -> Self {
        Self {
            cert_path: String::new(),
            key_path: String::new(),
            port: 443,
            bind_addr: "0.0.0.0".to_string(),
        }
    }
}

impl Http3Config {
    /// Create a new HTTP/3 configuration
    pub fn new(cert_path: impl Into<String>, key_path: impl Into<String>) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            ..Default::default()
        }
    }

    /// Set the port for HTTP/3 server
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the bind address
    pub fn bind_addr(mut self, addr: impl Into<String>) -> Self {
        self.bind_addr = addr.into();
        self
    }

    /// Get the full socket address
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.bind_addr, self.port)
    }
}

/// HTTP/3 Server using Quinn and h3
pub struct Http3Server {
    endpoint: Endpoint,
    router: Arc<Router>,
    layers: Arc<LayerStack>,
    interceptors: Arc<InterceptorChain>,
}

impl Http3Server {
    /// Create a new HTTP/3 server
    pub async fn new(
        config: &Http3Config,
        router: Arc<Router>,
        layers: Arc<LayerStack>,
        interceptors: Arc<InterceptorChain>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let server_config = Self::load_server_config(&config.cert_path, &config.key_path)?;
        let addr: SocketAddr = config.socket_addr().parse()?;
        let endpoint = Endpoint::server(server_config, addr)?;

        info!("🚀 HTTP/3 server bound to {}", addr);

        Ok(Self {
            endpoint,
            router,
            layers,
            interceptors,
        })
    }

    /// Create HTTP/3 server with self-signed certificate (development only)
    #[cfg(feature = "http3-dev")]
    pub async fn new_with_self_signed(
        addr: &str,
        router: Arc<Router>,
        layers: Arc<LayerStack>,
        interceptors: Arc<InterceptorChain>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (cert, key) = Self::generate_self_signed_cert()?;
        let server_config = Self::create_server_config(vec![cert], key)?;
        let addr: SocketAddr = addr.parse()?;
        let endpoint = Endpoint::server(server_config, addr)?;

        info!("🚀 HTTP/3 server (self-signed) bound to {}", addr);

        Ok(Self {
            endpoint,
            router,
            layers,
            interceptors,
        })
    }

    /// Run the HTTP/3 server
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.run_with_shutdown(std::future::pending()).await
    }

    /// Run the HTTP/3 server with graceful shutdown
    pub async fn run_with_shutdown<F>(
        self,
        signal: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        tokio::pin!(signal);

        loop {
            tokio::select! {
                Some(connecting) = self.endpoint.accept() => {
                    let router = self.router.clone();
                    let layers = self.layers.clone();
                    let interceptors = self.interceptors.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(connecting, router, layers, interceptors).await {
                            error!("HTTP/3 connection error: {}", e);
                        }
                    });
                }
                _ = &mut signal => {
                    info!("HTTP/3 shutdown signal received");
                    break;
                }
            }
        }

        // Close endpoint gracefully
        self.endpoint.close(0u32.into(), b"server shutdown");
        info!("HTTP/3 server shutdown complete");

        Ok(())
    }

    /// Handle a single QUIC connection
    async fn handle_connection(
        connecting: quinn::Incoming,
        router: Arc<Router>,
        layers: Arc<LayerStack>,
        interceptors: Arc<InterceptorChain>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connection = connecting.await?;
        let h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(connection)).await?;

        Self::handle_requests(h3_conn, router, layers, interceptors).await
    }

    /// Handle HTTP/3 requests on a connection
    async fn handle_requests(
        mut conn: h3::server::Connection<h3_quinn::Connection, Bytes>,
        router: Arc<Router>,
        layers: Arc<LayerStack>,
        interceptors: Arc<InterceptorChain>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            // h3 0.0.8 returns a RequestResolver instead of (Request, Stream)
            match conn.accept().await {
                Ok(Some(resolver)) => {
                    let router = router.clone();
                    let layers = layers.clone();
                    let interceptors = interceptors.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_request_resolver(resolver, router, layers, interceptors)
                                .await
                        {
                            error!("HTTP/3 request error: {}", e);
                        }
                    });
                }
                Ok(None) => {
                    // Connection closed
                    break;
                }
                Err(e) => {
                    error!("HTTP/3 accept error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a request resolver (h3 0.0.8 API)
    async fn handle_request_resolver(
        resolver: h3::server::RequestResolver<h3_quinn::Connection, Bytes>,
        router: Arc<Router>,
        layers: Arc<LayerStack>,
        interceptors: Arc<InterceptorChain>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Resolve the request to get the actual request and stream
        let (req, stream) = resolver.resolve_request().await?;
        Self::handle_request(req, stream, router, layers, interceptors).await
    }

    /// Handle a single HTTP/3 request
    async fn handle_request(
        req: http::Request<()>,
        mut stream: RequestStream<BidiStream<Bytes>, Bytes>,
        router: Arc<Router>,
        layers: Arc<LayerStack>,
        interceptors: Arc<InterceptorChain>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let start = std::time::Instant::now();

        // Read request body using Buf trait
        let mut body_bytes = Vec::new();
        while let Some(chunk) = stream.recv_data().await? {
            // chunk implements Buf, use remaining_slice or copy_to_slice
            let mut buf = chunk;
            while buf.has_remaining() {
                let chunk_slice = buf.chunk();
                body_bytes.extend_from_slice(chunk_slice);
                buf.advance(chunk_slice.len());
            }
        }

        // Convert to our Request type
        let (parts, _) = req.into_parts();
        let request = Request::new(
            parts,
            crate::request::BodyVariant::Buffered(Bytes::from(body_bytes)),
            router.state_ref(),
            crate::path_params::PathParams::new(),
        );

        // Apply request interceptors
        let request = interceptors.intercept_request(request);

        // Create routing handler
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
                    Box<
                        dyn std::future::Future<Output = crate::response::Response>
                            + Send
                            + 'static,
                    >,
                >
        });

        // Execute through middleware stack
        let response = layers.execute(request, routing_handler).await;

        // Apply response interceptors
        let response = interceptors.intercept_response(response);

        // Log request
        let elapsed = start.elapsed();
        if response.status().is_success() {
            info!(
                method = %method,
                path = %path,
                status = %response.status().as_u16(),
                duration_ms = %elapsed.as_millis(),
                protocol = "h3",
                "HTTP/3 request completed"
            );
        } else {
            error!(
                method = %method,
                path = %path,
                status = %response.status().as_u16(),
                duration_ms = %elapsed.as_millis(),
                protocol = "h3",
                "HTTP/3 request failed"
            );
        }

        // Send response
        let (parts, body) = response.into_parts();
        let http_response = http::Response::from_parts(parts, ());

        stream.send_response(http_response).await?;

        // Convert body to bytes and send
        use http_body_util::BodyExt;
        let collected = body
            .collect()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        let body_bytes = collected.to_bytes();
        stream.send_data(body_bytes).await?;

        stream.finish().await?;

        Ok(())
    }

    /// Load TLS configuration from PEM files
    fn load_server_config(
        cert_path: &str,
        key_path: &str,
    ) -> Result<ServerConfig, Box<dyn std::error::Error + Send + Sync>> {
        use std::fs::File;
        use std::io::BufReader;

        let cert_file = File::open(cert_path)?;
        let key_file = File::open(key_path)?;

        let certs: Vec<CertificateDer> =
            rustls_pemfile::certs(&mut BufReader::new(cert_file)).collect::<Result<Vec<_>, _>>()?;

        let key = rustls_pemfile::private_key(&mut BufReader::new(key_file))?
            .ok_or("No private key found")?;

        Self::create_server_config(certs, key)
    }

    /// Create Quinn server configuration from certificates
    fn create_server_config(
        certs: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
    ) -> Result<ServerConfig, Box<dyn std::error::Error + Send + Sync>> {
        let mut crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        crypto.alpn_protocols = vec![b"h3".to_vec()];

        let mut server_config = ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(crypto)?,
        ));

        // Configure transport parameters
        let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
        transport_config.max_concurrent_uni_streams(0_u8.into());
        transport_config.max_idle_timeout(Some(std::time::Duration::from_secs(30).try_into()?));

        Ok(server_config)
    }

    /// Generate a self-signed certificate for development
    #[cfg(feature = "http3-dev")]
    fn generate_self_signed_cert() -> Result<
        (CertificateDer<'static>, PrivateKeyDer<'static>),
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
        let key = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());
        let cert = CertificateDer::from(cert.cert.der().to_vec());

        Ok((cert, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http3_config_default() {
        let config = Http3Config::default();
        assert_eq!(config.port, 443);
        assert_eq!(config.bind_addr, "0.0.0.0");
    }

    #[test]
    fn test_http3_config_builder() {
        let config = Http3Config::new("cert.pem", "key.pem")
            .port(8443)
            .bind_addr("127.0.0.1");

        assert_eq!(config.cert_path, "cert.pem");
        assert_eq!(config.key_path, "key.pem");
        assert_eq!(config.port, 8443);
        assert_eq!(config.bind_addr, "127.0.0.1");
        assert_eq!(config.socket_addr(), "127.0.0.1:8443");
    }
}
