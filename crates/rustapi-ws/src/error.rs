//! WebSocket error types

use std::fmt;

/// Error type for WebSocket operations
#[derive(Debug)]
pub enum WebSocketError {
    /// Invalid WebSocket upgrade request
    InvalidUpgrade(String),
    /// WebSocket handshake failed
    HandshakeFailed(String),
    /// Connection closed unexpectedly
    ConnectionClosed,
    /// Failed to send message
    SendFailed(String),
    /// Failed to receive message
    ReceiveFailed(String),
    /// Message serialization error
    SerializationError(String),
    /// Message deserialization error
    DeserializationError(String),
    /// Protocol error
    ProtocolError(String),
    /// IO error
    IoError(std::io::Error),
    /// Tungstenite error
    Tungstenite(tungstenite::Error),
}

impl fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUpgrade(msg) => write!(f, "Invalid WebSocket upgrade request: {}", msg),
            Self::HandshakeFailed(msg) => write!(f, "WebSocket handshake failed: {}", msg),
            Self::ConnectionClosed => write!(f, "Connection closed unexpectedly"),
            Self::SendFailed(msg) => write!(f, "Failed to send message: {}", msg),
            Self::ReceiveFailed(msg) => write!(f, "Failed to receive message: {}", msg),
            Self::SerializationError(msg) => write!(f, "Message serialization error: {}", msg),
            Self::DeserializationError(msg) => write!(f, "Message deserialization error: {}", msg),
            Self::ProtocolError(msg) => write!(f, "WebSocket protocol error: {}", msg),
            Self::IoError(e) => write!(f, "IO error: {}", e),
            Self::Tungstenite(e) => write!(f, "WebSocket error: {}", e),
        }
    }
}

impl std::error::Error for WebSocketError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            Self::Tungstenite(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WebSocketError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl From<tungstenite::Error> for WebSocketError {
    fn from(e: tungstenite::Error) -> Self {
        Self::Tungstenite(e)
    }
}

impl WebSocketError {
    /// Create an invalid upgrade error
    pub fn invalid_upgrade(msg: impl Into<String>) -> Self {
        Self::InvalidUpgrade(msg.into())
    }

    /// Create a handshake failed error
    pub fn handshake_failed(msg: impl Into<String>) -> Self {
        Self::HandshakeFailed(msg.into())
    }

    /// Create a send failed error
    pub fn send_failed(msg: impl Into<String>) -> Self {
        Self::SendFailed(msg.into())
    }

    /// Create a receive failed error
    pub fn receive_failed(msg: impl Into<String>) -> Self {
        Self::ReceiveFailed(msg.into())
    }

    /// Create a serialization error
    pub fn serialization_error(msg: impl Into<String>) -> Self {
        Self::SerializationError(msg.into())
    }

    /// Create a deserialization error
    pub fn deserialization_error(msg: impl Into<String>) -> Self {
        Self::DeserializationError(msg.into())
    }

    /// Create a protocol error
    pub fn protocol_error(msg: impl Into<String>) -> Self {
        Self::ProtocolError(msg.into())
    }
}

impl From<WebSocketError> for rustapi_core::ApiError {
    fn from(err: WebSocketError) -> Self {
        match err {
            WebSocketError::InvalidUpgrade(msg) => {
                rustapi_core::ApiError::bad_request(format!("WebSocket upgrade failed: {}", msg))
            }
            WebSocketError::HandshakeFailed(msg) => {
                rustapi_core::ApiError::bad_request(format!("WebSocket handshake failed: {}", msg))
            }
            _ => rustapi_core::ApiError::internal(err.to_string()),
        }
    }
}

impl From<crate::auth::AuthError> for rustapi_core::ApiError {
    fn from(err: crate::auth::AuthError) -> Self {
        match err {
            crate::auth::AuthError::TokenMissing => {
                rustapi_core::ApiError::unauthorized("Authentication token missing")
            }
            crate::auth::AuthError::TokenExpired => {
                rustapi_core::ApiError::unauthorized("Token has expired")
            }
            crate::auth::AuthError::InvalidSignature => {
                rustapi_core::ApiError::unauthorized("Invalid token signature")
            }
            crate::auth::AuthError::InvalidFormat(msg) => {
                rustapi_core::ApiError::bad_request(format!("Invalid token format: {}", msg))
            }
            crate::auth::AuthError::ValidationFailed(msg) => {
                rustapi_core::ApiError::unauthorized(format!("Token validation failed: {}", msg))
            }
            crate::auth::AuthError::InsufficientPermissions(msg) => {
                rustapi_core::ApiError::forbidden(format!("Insufficient permissions: {}", msg))
            }
        }
    }
}
