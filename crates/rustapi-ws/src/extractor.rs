//! WebSocket extractor

use crate::upgrade::{validate_upgrade_request, WebSocketUpgrade};
use rustapi_core::{ApiError, FromRequestParts, Request, Result};
use rustapi_openapi::{Operation, OperationModifier};

/// WebSocket extractor for upgrading HTTP connections to WebSocket
///
/// Use this extractor in your handler to initiate a WebSocket upgrade.
/// The extractor validates the upgrade request and returns a `WebSocket`
/// that can be used to set up the connection handler.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_ws::{WebSocket, Message};
///
/// async fn ws_handler(ws: WebSocket) -> impl IntoResponse {
///     ws.on_upgrade(|socket| async move {
///         let (mut sender, mut receiver) = socket.split();
///         
///         while let Some(Ok(msg)) = receiver.next().await {
///             match msg {
///                 Message::Text(text) => {
///                     // Echo back
///                     let _ = sender.send(Message::text(format!("Echo: {}", text))).await;
///                 }
///                 Message::Close(_) => break,
///                 _ => {}
///             }
///         }
///     })
/// }
/// ```
pub struct WebSocket {
    sec_key: String,
    protocols: Vec<String>,
}

impl WebSocket {
    /// Create a WebSocket upgrade response with a handler
    ///
    /// The provided callback will be called with the established WebSocket
    /// stream once the upgrade is complete.
    pub fn on_upgrade<F, Fut>(self, callback: F) -> WebSocketUpgrade
    where
        F: FnOnce(crate::WebSocketStream) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let upgrade = WebSocketUpgrade::new(self.sec_key);
        
        // If protocols were requested, select the first one
        let upgrade = if let Some(protocol) = self.protocols.first() {
            upgrade.protocol(protocol)
        } else {
            upgrade
        };
        
        upgrade.on_upgrade(callback)
    }

    /// Get the requested protocols
    pub fn protocols(&self) -> &[String] {
        &self.protocols
    }

    /// Check if a specific protocol was requested
    pub fn has_protocol(&self, protocol: &str) -> bool {
        self.protocols.iter().any(|p| p == protocol)
    }
}

impl FromRequestParts for WebSocket {
    fn from_request_parts(req: &Request) -> Result<Self> {
        let headers = req.headers();
        let method = req.method();

        // Validate the upgrade request
        let sec_key = validate_upgrade_request(method, headers).map_err(|e| ApiError::from(e))?;

        // Parse requested protocols
        let protocols = headers
            .get("Sec-WebSocket-Protocol")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
            .unwrap_or_default();

        Ok(Self { sec_key, protocols })
    }
}

impl OperationModifier for WebSocket {
    fn update_operation(_op: &mut Operation) {
        // WebSocket endpoints don't have regular request body parameters
        // The upgrade is indicated by the response
    }
}
