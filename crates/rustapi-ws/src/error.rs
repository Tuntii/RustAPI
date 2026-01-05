//! WebSocket error types

use thiserror::Error;

/// Error type for WebSocket operations
#[derive(Error, Debug)]
pub enum WebSocketError {
    /// Invalid WebSocket upgrade request
    #[error("Invalid WebSocket upgrade request: {0}")]
    InvalidUpgrade(String),

    /// WebSocket handshake failed
    #[error("WebSocket handshake failed: {0}")]
    HandshakeFailed(String),

    /// Connection closed unexpectedly
    #[error("Connection closed unexpectedly")]
    ConnectionClosed,

    /// Failed to send message
    #[error("Failed to send message: {0}")]
    SendFailed(String),

    /// Failed to receive message
    #[error("Failed to receive message: {0}")]
    ReceiveFailed(String),

    /// Message serialization error
    #[error("Message serialization error: {0}")]
    SerializationError(String),

    /// Message deserialization error
    #[error("Message deserialization error: {0}")]
    DeserializationError(String),

    /// Protocol error
    #[error("WebSocket protocol error: {0}")]
    ProtocolError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Tungstenite error
    #[error("WebSocket error: {0}")]
    Tungstenite(#[from] tungstenite::Error),
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
