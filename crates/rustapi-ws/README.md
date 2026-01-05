# rustapi-ws

WebSocket support for RustAPI framework, enabling real-time bidirectional communication.

## Features

- **WebSocket Upgrade**: Seamless HTTP to WebSocket upgrade
- **Message Types**: Text, Binary, Ping/Pong support
- **Type-Safe Messages**: JSON serialization/deserialization
- **Connection Management**: Clean connection lifecycle handling
- **Broadcast Support**: Send messages to multiple clients

## Quick Start

```rust
use rustapi_rs::prelude::*;
use rustapi_ws::{WebSocket, Message};

async fn ws_handler(ws: WebSocket) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        let (mut sender, mut receiver) = socket.split();
        
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Echo the message back
                    let _ = sender.send(Message::Text(format!("Echo: {}", text))).await;
                }
                Ok(Message::Close(_)) => break,
                _ => {}
            }
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::new()
        .route("/ws", get(ws_handler))
        .run("127.0.0.1:8080")
        .await
}
```

## Message Types

```rust
use rustapi_ws::Message;

// Text message
let msg = Message::Text("Hello".to_string());

// Binary message
let msg = Message::Binary(vec![1, 2, 3]);

// JSON message (requires serde)
let msg = Message::json(&MyStruct { field: "value" })?;

// Ping/Pong
let msg = Message::Ping(vec![]);
let msg = Message::Pong(vec![]);

// Close connection
let msg = Message::Close(Some(CloseFrame {
    code: CloseCode::Normal,
    reason: "Goodbye".into(),
}));
```

## Connection State

```rust
use rustapi_ws::{WebSocket, WebSocketState};

async fn stateful_ws(ws: WebSocket, State(app_state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        // Access application state within the WebSocket handler
        let config = &app_state.config;
        // ...
    })
}
```

## Broadcasting

```rust
use rustapi_ws::{Broadcast, Message};
use std::sync::Arc;

// Create a broadcast channel
let broadcast = Arc::new(Broadcast::new());

// In your WebSocket handler
async fn ws_handler(ws: WebSocket, State(broadcast): State<Arc<Broadcast>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let (sender, mut receiver) = socket.split();
        
        // Subscribe to broadcasts
        let mut rx = broadcast.subscribe();
        
        // Handle incoming messages and broadcasts
        tokio::select! {
            // Receive from client
            msg = receiver.next() => {
                // Handle message
            }
            // Receive broadcast
            msg = rx.recv() => {
                // Forward to client
            }
        }
    })
}
```

## License

MIT OR Apache-2.0
