//! WebSocket upgrade response

use crate::{WebSocketError, WebSocketStream, WsHeartbeatConfig};
use http::{header, Response, StatusCode};
use hyper::upgrade::OnUpgrade;
use hyper_util::rt::TokioIo;
use rustapi_core::{IntoResponse, ResponseBody};
use rustapi_openapi::{Operation, ResponseModifier, ResponseSpec};
use std::collections::BTreeMap;
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

        if let Some(exts) = &self.client_extensions {
            if let Some(header_val) = negotiate_permessage_deflate(exts, config) {
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
                content: BTreeMap::new(),
                headers: BTreeMap::new(),
            },
        );
    }
}

#[derive(Debug)]
struct ParsedExtension {
    name: String,
    params: Vec<(String, Option<String>)>,
}

#[derive(Debug, Default)]
struct PerMessageDeflateOffer {
    server_no_context_takeover: bool,
    client_no_context_takeover: bool,
    server_max_window_bits: Option<Option<u8>>,
    client_max_window_bits: Option<Option<u8>>,
}

fn negotiate_permessage_deflate(
    client_extensions: &str,
    config: WsCompressionConfig,
) -> Option<String> {
    for ext in parse_extension_offers(client_extensions) {
        if ext.name != "permessage-deflate" {
            continue;
        }

        let Some(offer) = parse_permessage_deflate_offer(&ext) else {
            continue;
        };
        let mut negotiated = vec!["permessage-deflate".to_string()];

        if offer.server_no_context_takeover {
            negotiated.push("server_no_context_takeover".to_string());
        }
        if offer.client_no_context_takeover {
            negotiated.push("client_no_context_takeover".to_string());
        }

        if let Some(requested) = offer.server_max_window_bits {
            let bits = requested
                .map(|max| config.window_bits.min(max))
                .unwrap_or(config.window_bits);
            negotiated.push(format!("server_max_window_bits={}", bits));
        }

        if let Some(requested) = offer.client_max_window_bits {
            let bits = requested
                .map(|max| config.client_window_bits.min(max))
                .unwrap_or(config.client_window_bits);
            negotiated.push(format!("client_max_window_bits={}", bits));
        }

        return Some(negotiated.join("; "));
    }

    None
}

fn parse_extension_offers(header_value: &str) -> Vec<ParsedExtension> {
    let mut offers = Vec::new();

    for raw_extension in header_value.split(',') {
        let mut parts = raw_extension
            .split(';')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty());

        let Some(name) = parts.next() else {
            continue;
        };

        let mut params = Vec::new();
        for raw_param in parts {
            let (key, value) = parse_extension_param(raw_param);
            params.push((key, value));
        }

        offers.push(ParsedExtension {
            name: name.to_ascii_lowercase(),
            params,
        });
    }

    offers
}

fn parse_extension_param(raw_param: &str) -> (String, Option<String>) {
    if let Some((key, value)) = raw_param.split_once('=') {
        let value = value.trim().trim_matches('"').to_string();
        (key.trim().to_ascii_lowercase(), Some(value))
    } else {
        (raw_param.trim().to_ascii_lowercase(), None)
    }
}

fn parse_permessage_deflate_offer(ext: &ParsedExtension) -> Option<PerMessageDeflateOffer> {
    let mut offer = PerMessageDeflateOffer::default();

    for (key, value) in &ext.params {
        match key.as_str() {
            "server_no_context_takeover" => {
                if value.is_some() || offer.server_no_context_takeover {
                    return None;
                }
                offer.server_no_context_takeover = true;
            }
            "client_no_context_takeover" => {
                if value.is_some() || offer.client_no_context_takeover {
                    return None;
                }
                offer.client_no_context_takeover = true;
            }
            "server_max_window_bits" => {
                if offer.server_max_window_bits.is_some() {
                    return None;
                }
                let parsed = match value {
                    Some(v) => Some(parse_window_bits(v)?),
                    None => None,
                };
                offer.server_max_window_bits = Some(parsed);
            }
            "client_max_window_bits" => {
                if offer.client_max_window_bits.is_some() {
                    return None;
                }
                let parsed = match value {
                    Some(v) => Some(parse_window_bits(v)?),
                    None => None,
                };
                offer.client_max_window_bits = Some(parsed);
            }
            _ => {
                // Ignore unknown permessage-deflate params for compatibility.
            }
        }
    }

    Some(offer)
}

fn parse_window_bits(value: &str) -> Option<u8> {
    let parsed = value.parse::<u8>().ok()?;
    if (9..=15).contains(&parsed) {
        Some(parsed)
    } else {
        None
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
    use crate::WsCompressionConfig;

    #[test]
    fn test_accept_key_generation() {
        // Example from RFC 6455
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = generate_accept_key(key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn test_permessage_deflate_negotiates_context_takeover_and_window_bits() {
        let config = WsCompressionConfig::new()
            .window_bits(13)
            .client_window_bits(10);

        let negotiated = negotiate_permessage_deflate(
            "permessage-deflate; server_no_context_takeover; client_no_context_takeover; server_max_window_bits=12; client_max_window_bits",
            config,
        )
        .expect("expected successful negotiation");

        assert!(negotiated.contains("permessage-deflate"));
        assert!(negotiated.contains("server_no_context_takeover"));
        assert!(negotiated.contains("client_no_context_takeover"));
        assert!(negotiated.contains("server_max_window_bits=12"));
        assert!(negotiated.contains("client_max_window_bits=10"));
    }

    #[test]
    fn test_permessage_deflate_skips_invalid_offer_and_uses_next_offer() {
        let config = WsCompressionConfig::new()
            .window_bits(11)
            .client_window_bits(11);

        let negotiated = negotiate_permessage_deflate(
            "permessage-deflate; server_max_window_bits=7, permessage-deflate; client_max_window_bits",
            config,
        )
        .expect("expected fallback to second valid offer");

        assert!(negotiated.contains("permessage-deflate"));
        assert!(negotiated.contains("client_max_window_bits=11"));
        assert!(!negotiated.contains("server_max_window_bits=7"));
    }

    #[test]
    fn test_permessage_deflate_returns_none_when_not_offered() {
        let config = WsCompressionConfig::default();
        let negotiated = negotiate_permessage_deflate("x-webkit-deflate-frame", config);
        assert!(negotiated.is_none());
    }
}
