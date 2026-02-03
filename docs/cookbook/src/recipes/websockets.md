# Real-time Chat (WebSockets)

WebSockets allow full-duplex communication between the client and server. RustAPI leverages the `rustapi-ws` crate (based on `tungstenite` and `tokio`) to make this easy.

## Dependencies

```toml
[dependencies]
rustapi-ws = "0.1.275"
tokio = { version = "1", features = ["sync"] }
futures = "0.3"
```

## The Upgrade Handler

WebSocket connections start as HTTP requests. We "upgrade" them.

```rust
use rustapi_ws::{WebSocket, WebSocketUpgrade, Message};
use rustapi::prelude::*;
use std::sync::Arc;
use tokio::sync::broadcast;

// Shared state for broadcasting messages to all connected clients
pub struct AppState {
    pub tx: broadcast::Sender<String>,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Finalize the upgrade and spawn the socket handler
    ws.on_upgrade(|socket| handle_socket(socket, state))
}
```

## Handling the Connection

```rust
use futures::{sink::SinkExt, stream::StreamExt};

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    // Split the socket into a sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to the global broadcast channel
    let mut rx = state.tx.subscribe();

    // Spawn a task to forward broadcast messages to this client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // If the client disconnects, this will fail and we break
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages from THIS client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                println!("Received message: {}", text);
                // Broadcast it to everyone else
                let _ = state.tx.send(format!("User says: {}", text));
            }
        }
    });

    // Wait for either task to finish (disconnection)
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
```

## Initialization

```rust
#[tokio::main]
async fn main() {
    // Create a broadcast channel with capacity of 100 messages
    let (tx, _rx) = broadcast::channel(100);
    let state = Arc::new(AppState { tx });

    let app = RustApi::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    RustApi::serve("0.0.0.0:3000", app).await.unwrap();
}
```

## Client-Side Testing

You can simpler use JavaScript in the browser console:

```javascript
let ws = new WebSocket("ws://localhost:3000/ws");

ws.onmessage = (event) => {
    console.log("Message from server:", event.data);
};

ws.send("Hello form JS!");
```

## Advanced Patterns

1. **User Authentication**: Use the same `AuthUser` extractor in the `ws_handler`. If authentication fails, return an error *before* upgrading.
2. **Ping/Pong**: Browsers and Load Balancers kill idle connections. Implement a heartbeat mechanism to keep the connection alive.
    - `rustapi-ws` handles low-level ping/pong frames automatically in many cases, but application-level pings are also robust.
