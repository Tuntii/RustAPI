//! WebSocket message types

use serde::{de::DeserializeOwned, Serialize};
use std::borrow::Cow;

/// WebSocket message type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Text message (UTF-8 encoded)
    Text(String),
    /// Binary message
    Binary(Vec<u8>),
    /// Ping message
    Ping(Vec<u8>),
    /// Pong message
    Pong(Vec<u8>),
    /// Close message
    Close(Option<CloseFrame>),
}

impl Message {
    /// Create a text message
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Create a binary message
    pub fn binary(data: impl Into<Vec<u8>>) -> Self {
        Self::Binary(data.into())
    }

    /// Create a ping message
    pub fn ping(data: impl Into<Vec<u8>>) -> Self {
        Self::Ping(data.into())
    }

    /// Create a pong message
    pub fn pong(data: impl Into<Vec<u8>>) -> Self {
        Self::Pong(data.into())
    }

    /// Create a close message
    pub fn close() -> Self {
        Self::Close(None)
    }

    /// Create a close message with a frame
    pub fn close_with(code: CloseCode, reason: impl Into<String>) -> Self {
        Self::Close(Some(CloseFrame {
            code,
            reason: Cow::Owned(reason.into()),
        }))
    }

    /// Create a JSON text message from a serializable type
    pub fn json<T: Serialize>(value: &T) -> Result<Self, crate::WebSocketError> {
        serde_json::to_string(value)
            .map(Self::Text)
            .map_err(|e| crate::WebSocketError::serialization_error(e.to_string()))
    }

    /// Try to deserialize a text message as JSON
    pub fn as_json<T: DeserializeOwned>(&self) -> Result<T, crate::WebSocketError> {
        match self {
            Self::Text(text) => serde_json::from_str(text)
                .map_err(|e| crate::WebSocketError::deserialization_error(e.to_string())),
            _ => Err(crate::WebSocketError::deserialization_error(
                "Expected text message for JSON deserialization",
            )),
        }
    }

    /// Check if this is a text message
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Check if this is a binary message
    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary(_))
    }

    /// Check if this is a ping message
    pub fn is_ping(&self) -> bool {
        matches!(self, Self::Ping(_))
    }

    /// Check if this is a pong message
    pub fn is_pong(&self) -> bool {
        matches!(self, Self::Pong(_))
    }

    /// Check if this is a close message
    pub fn is_close(&self) -> bool {
        matches!(self, Self::Close(_))
    }

    /// Get the text content if this is a text message
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Get the binary content if this is a binary message
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Binary(data) => Some(data),
            _ => None,
        }
    }

    /// Convert to text, consuming the message
    pub fn into_text(self) -> Option<String> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Convert to bytes, consuming the message
    pub fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            Self::Binary(data) => Some(data),
            _ => None,
        }
    }
}

impl From<String> for Message {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for Message {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<Vec<u8>> for Message {
    fn from(data: Vec<u8>) -> Self {
        Self::Binary(data)
    }
}

impl From<&[u8]> for Message {
    fn from(data: &[u8]) -> Self {
        Self::Binary(data.to_vec())
    }
}

/// Convert from tungstenite Message
impl From<tungstenite::Message> for Message {
    fn from(msg: tungstenite::Message) -> Self {
        match msg {
            tungstenite::Message::Text(text) => Self::Text(text.to_string()),
            tungstenite::Message::Binary(data) => Self::Binary(data.to_vec()),
            tungstenite::Message::Ping(data) => Self::Ping(data.to_vec()),
            tungstenite::Message::Pong(data) => Self::Pong(data.to_vec()),
            tungstenite::Message::Close(frame) => Self::Close(frame.map(|f| CloseFrame {
                code: CloseCode::from(f.code),
                reason: Cow::Owned(f.reason.to_string()),
            })),
            tungstenite::Message::Frame(_) => Self::Binary(vec![]), // Raw frames treated as binary
        }
    }
}

/// Convert to tungstenite Message
impl From<Message> for tungstenite::Message {
    fn from(msg: Message) -> Self {
        match msg {
            Message::Text(text) => tungstenite::Message::Text(text),
            Message::Binary(data) => tungstenite::Message::Binary(data),
            Message::Ping(data) => tungstenite::Message::Ping(data),
            Message::Pong(data) => tungstenite::Message::Pong(data),
            Message::Close(frame) => {
                tungstenite::Message::Close(frame.map(|f| tungstenite::protocol::CloseFrame {
                    code: f.code.into(),
                    reason: f.reason,
                }))
            }
        }
    }
}

/// WebSocket close frame
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseFrame {
    /// Close code
    pub code: CloseCode,
    /// Close reason
    pub reason: Cow<'static, str>,
}

impl CloseFrame {
    /// Create a new close frame
    pub fn new(code: CloseCode, reason: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code,
            reason: reason.into(),
        }
    }

    /// Create a normal close frame
    pub fn normal() -> Self {
        Self::new(CloseCode::Normal, "")
    }

    /// Create a going away close frame
    pub fn going_away() -> Self {
        Self::new(CloseCode::Away, "Going away")
    }
}

/// WebSocket close codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CloseCode {
    /// Normal closure (1000)
    Normal,
    /// Going away (1001)
    Away,
    /// Protocol error (1002)
    Protocol,
    /// Unsupported data (1003)
    Unsupported,
    /// No status received (1005)
    Status,
    /// Abnormal closure (1006)
    Abnormal,
    /// Invalid frame payload data (1007)
    Invalid,
    /// Policy violation (1008)
    Policy,
    /// Message too big (1009)
    Size,
    /// Mandatory extension (1010)
    Extension,
    /// Internal error (1011)
    Error,
    /// Service restart (1012)
    Restart,
    /// Try again later (1013)
    Again,
    /// Bad TLS handshake (1015)
    Tls,
    /// Reserved codes
    Reserved(u16),
    /// Library/framework-specific codes (3000-3999)
    Library(u16),
    /// Private use codes (4000-4999)
    Private(u16),
}

impl CloseCode {
    /// Get the numeric code
    pub fn as_u16(&self) -> u16 {
        match self {
            Self::Normal => 1000,
            Self::Away => 1001,
            Self::Protocol => 1002,
            Self::Unsupported => 1003,
            Self::Status => 1005,
            Self::Abnormal => 1006,
            Self::Invalid => 1007,
            Self::Policy => 1008,
            Self::Size => 1009,
            Self::Extension => 1010,
            Self::Error => 1011,
            Self::Restart => 1012,
            Self::Again => 1013,
            Self::Tls => 1015,
            Self::Reserved(code) => *code,
            Self::Library(code) => *code,
            Self::Private(code) => *code,
        }
    }
}

impl From<u16> for CloseCode {
    fn from(code: u16) -> Self {
        match code {
            1000 => Self::Normal,
            1001 => Self::Away,
            1002 => Self::Protocol,
            1003 => Self::Unsupported,
            1005 => Self::Status,
            1006 => Self::Abnormal,
            1007 => Self::Invalid,
            1008 => Self::Policy,
            1009 => Self::Size,
            1010 => Self::Extension,
            1011 => Self::Error,
            1012 => Self::Restart,
            1013 => Self::Again,
            1015 => Self::Tls,
            1004 | 1014 | 1016..=2999 => Self::Reserved(code),
            3000..=3999 => Self::Library(code),
            4000..=4999 => Self::Private(code),
            _ => Self::Reserved(code),
        }
    }
}

impl From<CloseCode> for u16 {
    fn from(code: CloseCode) -> Self {
        code.as_u16()
    }
}

impl From<tungstenite::protocol::frame::coding::CloseCode> for CloseCode {
    fn from(code: tungstenite::protocol::frame::coding::CloseCode) -> Self {
        Self::from(u16::from(code))
    }
}

impl From<CloseCode> for tungstenite::protocol::frame::coding::CloseCode {
    fn from(code: CloseCode) -> Self {
        tungstenite::protocol::frame::coding::CloseCode::from(code.as_u16())
    }
}
