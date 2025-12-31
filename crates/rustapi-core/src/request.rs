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

use bytes::Bytes;
use http::{request::Parts, Extensions, HeaderMap, Method, Uri, Version};
use std::collections::HashMap;
use std::sync::Arc;

/// HTTP Request wrapper
///
/// Provides access to all parts of an incoming HTTP request.
pub struct Request {
    pub(crate) parts: Parts,
    pub(crate) body: Option<Bytes>,
    pub(crate) state: Arc<Extensions>,
    pub(crate) path_params: HashMap<String, String>,
}

impl Request {
    /// Create a new request from parts
    pub(crate) fn new(
        parts: Parts,
        body: Bytes,
        state: Arc<Extensions>,
        path_params: HashMap<String, String>,
    ) -> Self {
        Self {
            parts,
            body: Some(body),
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
    pub fn take_body(&mut self) -> Option<Bytes> {
        self.body.take()
    }

    /// Get path parameters
    pub fn path_params(&self) -> &HashMap<String, String> {
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

    /// Create a test request from an http::Request
    /// 
    /// This is useful for testing middleware and extractors.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn from_http_request<B>(req: http::Request<B>, body: Bytes) -> Self {
        let (parts, _) = req.into_parts();
        Self {
            parts,
            body: Some(body),
            state: Arc::new(Extensions::new()),
            path_params: HashMap::new(),
        }
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
