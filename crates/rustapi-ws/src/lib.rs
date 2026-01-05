//! # rustapi-ws
//!
//! WebSocket support for the RustAPI framework.
//!
//! This crate provides WebSocket upgrade handling, message types, and utilities
//! for building real-time bidirectional communication in your RustAPI applications.
//!
//! ## Features
//!
//! - **WebSocket Upgrade**: Seamless HTTP to WebSocket upgrade via the `WebSocket` extractor
//! - **Message Types**: Support for Text, Binary, Ping/Pong messages
//! - **Type-Safe JSON**: Serialize/deserialize JSON messages with serde
//! - **Connection Management**: Clean connection lifecycle with proper close handling
//! - **Broadcast Support**: Send messages to multiple connected clients
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rustapi_rs::prelude::*;
//! use rustapi_ws::{WebSocket, Message};
//!
//! async fn ws_handler(ws: WebSocket) -> impl IntoResponse {
//!     ws.on_upgrade(|socket| async move {
//!         let (mut sender, mut receiver) = socket.split();
//!         
//!         while let Some(msg) = receiver.next().await {
//!             match msg {
//!                 Ok(Message::Text(text)) => {
//!                     let _ = sender.send(Message::Text(format!("Echo: {}", text))).await;
//!                 }
//!                 Ok(Message::Close(_)) => break,
//!                 _ => {}
//!             }
//!         }
//!     })
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     RustApi::new()
//!         .route("/ws", get(ws_handler))
//!         .run("127.0.0.1:8080")
//!         .await
//! }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod error;
mod extractor;
mod message;
mod socket;
mod broadcast;
mod upgrade;

pub use error::WebSocketError;
pub use extractor::WebSocket;
pub use message::{CloseCode, CloseFrame, Message};
pub use socket::{WebSocketReceiver, WebSocketSender, WebSocketStream};
pub use broadcast::Broadcast;
pub use upgrade::WebSocketUpgrade;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Broadcast,
        CloseCode,
        CloseFrame,
        Message,
        WebSocket,
        WebSocketError,
        WebSocketReceiver,
        WebSocketSender,
        WebSocketStream,
        WebSocketUpgrade,
    };
}
