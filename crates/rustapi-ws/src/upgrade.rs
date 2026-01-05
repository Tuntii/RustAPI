//! WebSocket upgrade response

use crate::{WebSocketError, WebSocketStream};
use bytes::Bytes;
use http::{header, Response, StatusCode};
use http_body_util::Full;
use rustapi_core::IntoResponse;
use rustapi_openapi::{Operation, ResponseModifier, ResponseSpec};
use std::future::Future;
use std::pin::Pin;

/// WebSocket upgrade response
///
/// This type is returned from WebSocket handlers to initiate the upgrade
/// handshake and establish a WebSocket connection.
pub struct WebSocketUpgrade {
    /// The upgrade response
    response: Response<Full<Bytes>>,
    /// Callback to handle the WebSocket connection
    on_upgrade: Option<
        Box<dyn FnOnce(WebSocketStream) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>,
    >,
    /// SEC-WebSocket-Key from request
    sec_key: String,
}

impl WebSocketUpgrade {
    /// Create a new WebSocket upgrade from request headers
    pub(crate) fn new(sec_key: String) -> Self {
        // Generate accept key
        let accept_key = generate_accept_key(&sec_key);

        // Build upgrade response
        let response = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(header::UPGRADE, "websocket")
            .header(header::CONNECTION, "Upgrade")
            .header("Sec-WebSocket-Accept", accept_key)
            .body(Full::new(Bytes::new()))
            .unwrap();

        Self {
            response,
            on_upgrade: None,
            sec_key,
        }
    }

    /// Set the callback to handle the upgraded WebSocket connection
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// ws.on_upgrade(|socket| async move {
    ///     let (mut sender, mut receiver) = socket.split();
    ///     while let Some(msg) = receiver.next().await {
    ///         // Handle messages...
    ///     }
    /// })
    /// ```
    pub fn on_upgrade<F, Fut>(mut self, callback: F) -> Self
    where
        F: FnOnce(WebSocketStream) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_upgrade = Some(Box::new(move |stream| Box::pin(callback(stream))));
        self
    }

    /// Add a protocol to the response
    pub fn protocol(mut self, protocol: &str) -> Self {
        self.response = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(header::UPGRADE, "websocket")
            .header(header::CONNECTION, "Upgrade")
            .header("Sec-WebSocket-Accept", generate_accept_key(&self.sec_key))
            .header("Sec-WebSocket-Protocol", protocol)
            .body(Full::new(Bytes::new()))
            .unwrap();
        self
    }

    /// Get the underlying response (for implementing IntoResponse)
    #[allow(dead_code)]
    pub(crate) fn into_response_inner(self) -> Response<Full<Bytes>> {
        self.response
    }

    /// Get the on_upgrade callback
    #[allow(dead_code)]
    pub(crate) fn take_callback(
        &mut self,
    ) -> Option<Box<dyn FnOnce(WebSocketStream) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>>
    {
        self.on_upgrade.take()
    }
}

impl IntoResponse for WebSocketUpgrade {
    fn into_response(self) -> http::Response<Full<Bytes>> {
        self.response
    }
}

impl ResponseModifier for WebSocketUpgrade {
    fn update_response(op: &mut Operation) {
        op.responses.insert(
            "101".to_string(),
            ResponseSpec {
                description: "WebSocket upgrade successful".to_string(),
                content: None,
            },
        );
    }
}

/// Generate the Sec-WebSocket-Accept key from the client's Sec-WebSocket-Key
fn generate_accept_key(key: &str) -> String {
    use base64::Engine;
    use sha1::{Digest, Sha1};

    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(GUID.as_bytes());
    let hash = hasher.finalize();

    base64::engine::general_purpose::STANDARD.encode(hash)
}

/// Validate that a request is a valid WebSocket upgrade request
pub(crate) fn validate_upgrade_request(
    method: &http::Method,
    headers: &http::HeaderMap,
) -> Result<String, WebSocketError> {
    // Must be GET
    if method != http::Method::GET {
        return Err(WebSocketError::invalid_upgrade("Method must be GET"));
    }

    // Must have Upgrade: websocket header
    let upgrade = headers
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| WebSocketError::invalid_upgrade("Missing Upgrade header"))?;

    if !upgrade.eq_ignore_ascii_case("websocket") {
        return Err(WebSocketError::invalid_upgrade(
            "Upgrade header must be 'websocket'",
        ));
    }

    // Must have Connection: Upgrade header
    let connection = headers
        .get(header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| WebSocketError::invalid_upgrade("Missing Connection header"))?;

    let has_upgrade = connection
        .split(',')
        .any(|s| s.trim().eq_ignore_ascii_case("upgrade"));

    if !has_upgrade {
        return Err(WebSocketError::invalid_upgrade(
            "Connection header must contain 'Upgrade'",
        ));
    }

    // Must have Sec-WebSocket-Key header
    let sec_key = headers
        .get("Sec-WebSocket-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| WebSocketError::invalid_upgrade("Missing Sec-WebSocket-Key header"))?;

    // Must have Sec-WebSocket-Version: 13
    let version = headers
        .get("Sec-WebSocket-Version")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| WebSocketError::invalid_upgrade("Missing Sec-WebSocket-Version header"))?;

    if version != "13" {
        return Err(WebSocketError::invalid_upgrade(
            "Sec-WebSocket-Version must be 13",
        ));
    }

    Ok(sec_key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accept_key_generation() {
        // Example from RFC 6455
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = generate_accept_key(key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}
