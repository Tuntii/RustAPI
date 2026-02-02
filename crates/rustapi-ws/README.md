# rustapi-ws

**Lens**: "The Live Wire"  
**Philosophy**: "Real-time, persistent connections made simple."

Real-time bidirectional communication for RustAPI, built on `tokio-tungstenite`.

## The WebSocket Extractor

Upgrading an HTTP connection to a WebSocket uses the standard extractor pattern:

```rust
async fn ws_handler(
    ws: WebSocket,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}
```

## Architecture

We recommend an **Actor Model** for WebSocket state:
1. Each connection spawns a new async task (the actor)
2. Use `tokio::sync::broadcast` channels for global events (like chat rooms)
3. Use `mpsc` channels for direct messaging

## Features
- **Auto-Upgrade**: Handles the HTTP 101 Switching Protocols handshake
- **Channels**: Built-in pub/sub for broadcast scenarios (chat rooms)
- **Ping/Pong**: Automatic heartbeat management

## Full Example

```rust
use rustapi_ws::{WebSocket, Message};

#[rustapi_rs::get("/chat")]
async fn chat_handler(ws: WebSocket) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            println!("Received: {}", text);
            socket.send(Message::Text("Echo!".into())).await.unwrap();
        }
    }
}
```
