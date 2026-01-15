//! Request types for RustAPI
//!
//! This module provides the [`Request`] type which wraps an incoming HTTP request
//! and provides access to all its components.
//!
//! # Accessing Request Data
//!
//! While extractors are the preferred way to access request data in handlers,
//! the `Request` type provides direct access when needed:
//!
//! ```rust,ignore
//! // In middleware or custom extractors
//! fn process_request(req: &Request) {
//!     let method = req.method();
//!     let path = req.path();
//!     let headers = req.headers();
//!     let query = req.query_string();
//! }
//! ```
//!
//! # Path Parameters
//!
//! Path parameters extracted from the URL pattern are available via:
//!
//! ```rust,ignore
//! // For route "/users/{id}"
//! let id = req.path_param("id");
//! let all_params = req.path_params();
//! ```
//!
//! # Request Body
//!
//! The body can only be consumed once:
//!
//! ```rust,ignore
//! if let Some(body) = req.take_body() {
//!     // Process body bytes
//! }
//! // Subsequent calls return None
//! ```

use crate::path_params::PathParams;
use bytes::Bytes;
use http::{request::Parts, Extensions, HeaderMap, Method, Uri, Version};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use std::sync::Arc;

/// Internal representation of the request body state
pub(crate) enum BodyVariant {
    Buffered(Bytes),
    Streaming(Incoming),
    Consumed,
}

/// HTTP Request wrapper
///
/// Provides access to all parts of an incoming HTTP request.
pub struct Request {
    pub(crate) parts: Parts,
    pub(crate) body: BodyVariant,
    pub(crate) state: Arc<Extensions>,
    pub(crate) path_params: PathParams,
}

impl Request {
    /// Create a new request from parts
    pub(crate) fn new(
        parts: Parts,
        body: BodyVariant,
        state: Arc<Extensions>,
        path_params: PathParams,
    ) -> Self {
        Self {
            parts,
            body,
            state,
            path_params,
        }
    }

    /// Get the HTTP method
    pub fn method(&self) -> &Method {
        &self.parts.method
    }

    /// Get the URI
    pub fn uri(&self) -> &Uri {
        &self.parts.uri
    }

    /// Get the HTTP version
    pub fn version(&self) -> Version {
        self.parts.version
    }

    /// Get the headers
    pub fn headers(&self) -> &HeaderMap {
        &self.parts.headers
    }

    /// Get request extensions
    pub fn extensions(&self) -> &Extensions {
        &self.parts.extensions
    }

    /// Get mutable extensions
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.parts.extensions
    }

    /// Get the request path
    pub fn path(&self) -> &str {
        self.parts.uri.path()
    }

    /// Get the query string
    pub fn query_string(&self) -> Option<&str> {
        self.parts.uri.query()
    }

    /// Take the body bytes (can only be called once)
    ///
    /// Returns None if the body is streaming or already consumed.
    /// Use `load_body().await` first if you need to ensure the body is available as bytes.
    pub fn take_body(&mut self) -> Option<Bytes> {
        match std::mem::replace(&mut self.body, BodyVariant::Consumed) {
            BodyVariant::Buffered(bytes) => Some(bytes),
            other => {
                self.body = other;
                None
            }
        }
    }

    /// Take the body as a stream (can only be called once)
    pub fn take_stream(&mut self) -> Option<Incoming> {
        match std::mem::replace(&mut self.body, BodyVariant::Consumed) {
            BodyVariant::Streaming(stream) => Some(stream),
            other => {
                self.body = other;
                None
            }
        }
    }

    /// Ensure the body is loaded into memory.
    ///
    /// If the body is streaming, this collects it into Bytes.
    /// If already buffered, does nothing.
    /// Returns error if collection fails.
    pub async fn load_body(&mut self) -> Result<(), crate::error::ApiError> {
        // We moved the body out to check, put it back if buffered or new buffer
        let new_body = match std::mem::replace(&mut self.body, BodyVariant::Consumed) {
            BodyVariant::Streaming(incoming) => {
                let collected = incoming
                    .collect()
                    .await
                    .map_err(|e| crate::error::ApiError::bad_request(e.to_string()))?;
                BodyVariant::Buffered(collected.to_bytes())
            }
            BodyVariant::Buffered(b) => BodyVariant::Buffered(b),
            BodyVariant::Consumed => BodyVariant::Consumed,
        };
        self.body = new_body;
        Ok(())
    }

    /// Get path parameters
    pub fn path_params(&self) -> &PathParams {
        &self.path_params
    }

    /// Get a specific path parameter
    pub fn path_param(&self, name: &str) -> Option<&String> {
        self.path_params.get(name)
    }

    /// Get shared state
    pub fn state(&self) -> &Arc<Extensions> {
        &self.state
    }

    /// Set path parameters (used internally after route matching)
    pub(crate) fn set_path_params(&mut self, params: PathParams) {
        self.path_params = params;
    }

    /// Create a test request from an http::Request
    ///
    /// This is useful for testing middleware and extractors.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn from_http_request<B>(req: http::Request<B>, body: Bytes) -> Self {
        let (parts, _) = req.into_parts();
        Self {
            parts,
            body: BodyVariant::Buffered(body),
            state: Arc::new(Extensions::new()),
            path_params: PathParams::new(),
        }
    }
    /// Try to clone the request.
    ///
    /// This creates a deep copy of the request, including headers, body (if present),
    /// path params, and shared state.
    ///
    /// Returns None if the body is streaming (cannot be cloned) or already consumed.
    pub fn try_clone(&self) -> Option<Self> {
        let mut builder = http::Request::builder()
            .method(self.method().clone())
            .uri(self.uri().clone())
            .version(self.version());

        if let Some(headers) = builder.headers_mut() {
            *headers = self.headers().clone();
        }

        let req = builder.body(()).ok()?;
        let (parts, _) = req.into_parts();

        let new_body = match &self.body {
            BodyVariant::Buffered(b) => BodyVariant::Buffered(b.clone()),
            BodyVariant::Streaming(_) => return None, // Cannot clone stream
            BodyVariant::Consumed => return None,
        };

        Some(Self {
            parts,
            body: new_body,
            state: self.state.clone(),
            path_params: self.path_params.clone(),
        })
    }
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.parts.method)
            .field("uri", &self.parts.uri)
            .field("version", &self.parts.version)
            .finish()
    }
}
