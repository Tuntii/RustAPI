//! WebSocket upgrade response

use crate::{WebSocketError, WebSocketStream, WsHeartbeatConfig};
use bytes::Bytes;
use http::{header, Response, StatusCode};
use hyper::upgrade::OnUpgrade;
use hyper_util::rt::TokioIo;
use rustapi_core::{ApiError, IntoResponse, ResponseBody};
use rustapi_openapi::{Operation, ResponseModifier, ResponseSpec};
use std::future::Future;
use std::pin::Pin;
use tokio_tungstenite::tungstenite::protocol::Role;

/// Type alias for WebSocket upgrade callback
type UpgradeCallback =
    Box<dyn FnOnce(WebSocketStream) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// WebSocket upgrade response
///
/// This type is returned from WebSocket handlers to initiate the upgrade
/// handshake and establish a WebSocket connection.
use crate::compression::WsCompressionConfig;

/// WebSocket upgrade response
///
/// This type is returned from WebSocket handlers to initiate the upgrade
/// handshake and establish a WebSocket connection.
pub struct WebSocketUpgrade {
    /// The upgrade response
    response: Response<ResponseBody>,
    /// Callback to handle the WebSocket connection
    on_upgrade: Option<UpgradeCallback>,
    /// SEC-WebSocket-Key from request
    #[allow(dead_code)]
    sec_key: String,
    /// Client requested extensions
    client_extensions: Option<String>,
    /// Configured compression
    compression: Option<WsCompressionConfig>,
    /// Configured heartbeat
    heartbeat: Option<WsHeartbeatConfig>,
    /// OnUpgrade future from hyper
    on_upgrade_fut: Option<OnUpgrade>,
}

impl WebSocketUpgrade {
    /// Create a new WebSocket upgrade from request headers
    pub(crate) fn new(
        sec_key: String,
        client_extensions: Option<String>,
        on_upgrade_fut: Option<OnUpgrade>,
    ) -> Self {
        // Generate accept key
        let accept_key = generate_accept_key(&sec_key);

        // Build upgrade response
        let response = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(header::UPGRADE, "websocket")
            .header(header::CONNECTION, "Upgrade")
            .header("Sec-WebSocket-Accept", accept_key)
            .body(ResponseBody::empty())
            .unwrap();

        Self {
            response,
            on_upgrade: None,
            sec_key,
            client_extensions,
            compression: None,
            heartbeat: None,
            on_upgrade_fut,
        }
    }

    /// Enable WebSocket heartbeat
    pub fn heartbeat(mut self, config: WsHeartbeatConfig) -> Self {
        self.heartbeat = Some(config);
        self
    }

    /// Enable WebSocket compression
    pub fn compress(mut self, config: WsCompressionConfig) -> Self {
        self.compression = Some(config);

        // Simple negotiation: if client supports it, we enable it
        if let Some(exts) = &self.client_extensions {
            if exts.contains("permessage-deflate") {
                // We currently use a simple negotiation strategy
                // TODO: Parse parameters and negotiate window bits
                let mut header_val = String::from("permessage-deflate");

                // Add server/client_no_context_takeover to reduce memory usage at cost of compression ratio
                // This is a common default for many servers
                header_val.push_str("; server_no_context_takeover");
                header_val.push_str("; client_no_context_takeover");

                if config.window_bits < 15 {
                    header_val
                        .push_str(&format!("; server_max_window_bits={}", config.window_bits));
                }
                if config.client_window_bits < 15 {
                    header_val.push_str(&format!(
                        "; client_max_window_bits={}",
                        config.client_window_bits
                    ));
                }

                if let Ok(val) = header::HeaderValue::from_str(&header_val) {
                    self.response
                        .headers_mut()
                        .insert("Sec-WebSocket-Extensions", val);
                }
            }
        }
        self
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
        // Rebuild response to keep headers clean (or just insert)
        // More efficient to just insert
        self.response.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            header::HeaderValue::from_str(protocol).unwrap(),
        );
        self
    }

    /// Get the underlying response (for implementing IntoResponse)
    #[allow(dead_code)]
    pub(crate) fn into_response_inner(self) -> Response<ResponseBody> {
        self.response
    }

    /// Get the on_upgrade callback
    #[allow(dead_code)]
    pub(crate) fn take_callback(&mut self) -> Option<UpgradeCallback> {
        self.on_upgrade.take()
    }
}

impl IntoResponse for WebSocketUpgrade {
    fn into_response(mut self) -> rustapi_core::Response {
        // If we have the upgrade future and a callback, spawn the upgrade task
        if let (Some(on_upgrade), Some(callback)) =
            (self.on_upgrade_fut.take(), self.on_upgrade.take())
        {
            let heartbeat = self.heartbeat;

            // TODO: Apply compression config to WebSocketConfig if/when supported by from_raw_socket
            // Currently tungstenite negotiation logic in handshake is separate from stream config

            tokio::spawn(async move {
                match on_upgrade.await {
                    Ok(upgraded) => {
                        let ws_stream = tokio_tungstenite::WebSocketStream::from_raw_socket(
                            TokioIo::new(upgraded),
                            Role::Server,
                            None,
                        )
                        .await;

                        let socket = if let Some(hb_config) = heartbeat {
                            WebSocketStream::new_managed(ws_stream, hb_config)
                        } else {
                            WebSocketStream::new(ws_stream)
                        };

                        callback(socket).await;
                    }
                    Err(e) => {
                        tracing::error!("WebSocket upgrade failed: {:?}", e);
                        // Also try to print the source if available
                        if let Some(source) = std::error::Error::source(&e) {
                            tracing::error!("Cause: {:?}", source);
                        }
                    }
                }
            });
        }

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
