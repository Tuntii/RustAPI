//! WebSocket Example
//!
//! This example demonstrates WebSocket support in RustAPI:
//! - Basic echo server
//! - JSON message handling
//! - Broadcast to multiple clients
//!
//! Run with: cargo run --package websocket-example
//! Test with a WebSocket client (e.g., websocat):
//!   websocat ws://localhost:8080/ws/echo
//!   websocat ws://localhost:8080/ws/chat

use rustapi_rs::prelude::*;
use rustapi_rs::ws::{Broadcast, Message, WebSocket, WebSocketUpgrade};
use std::sync::Arc;

/// Chat message for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    username: String,
    content: String,
    timestamp: u64,
}

/// Application state containing the broadcast channel
struct AppState {
    chat_broadcast: Arc<Broadcast>,
}

/// Simple echo WebSocket endpoint
async fn ws_echo(ws: WebSocket) -> WebSocketUpgrade {
    ws.on_upgrade(|mut socket| async move {
        tracing::info!("New echo connection");

        while let Some(result) = socket.recv().await {
            match result {
                Ok(Message::Text(text)) => {
                    tracing::debug!("Received: {}", text);
                    if let Err(e) = socket.send(Message::text(format!("Echo: {}", text))).await {
                        tracing::error!("Send error: {}", e);
                        break;
                    }
                }
                Ok(Message::Binary(data)) => {
                    if let Err(e) = socket.send(Message::binary(data)).await {
                        tracing::error!("Send error: {}", e);
                        break;
                    }
                }
                Ok(Message::Ping(data)) => {
                    let _ = socket.send(Message::pong(data)).await;
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Client disconnected");
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Receive error: {}", e);
                    break;
                }
            }
        }
    })
}

/// JSON echo WebSocket endpoint
async fn ws_json(ws: WebSocket) -> WebSocketUpgrade {
    ws.on_upgrade(|mut socket| async move {
        tracing::info!("New JSON connection");

        while let Some(result) = socket.recv().await {
            match result {
                Ok(msg) => {
                    if msg.is_text() {
                        // Try to parse as ChatMessage
                        match msg.as_json::<ChatMessage>() {
                            Ok(chat_msg) => {
                                tracing::info!(
                                    "Message from {}: {}",
                                    chat_msg.username,
                                    chat_msg.content
                                );

                                // Echo back with modified content
                                let response = ChatMessage {
                                    username: "server".to_string(),
                                    content: format!("Received: {}", chat_msg.content),
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                };

                                if let Err(e) = socket.send_json(&response).await {
                                    tracing::error!("Send error: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Invalid JSON: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Receive error: {}", e);
                    break;
                }
            }
        }
    })
}

/// Chat room WebSocket endpoint with broadcasting
async fn ws_chat(ws: WebSocket, State(state): State<Arc<AppState>>) -> WebSocketUpgrade {
    ws.on_upgrade(move |socket| async move {
        let (mut sender, mut receiver) = socket.split();
        let broadcast = state.chat_broadcast.clone();

        // Subscribe to broadcast messages
        let mut broadcast_rx = broadcast.subscribe();

        tracing::info!(
            "New chat connection (total: {})",
            broadcast.subscriber_count()
        );

        // Announce new user
        let _ = broadcast.send_json(&ChatMessage {
            username: "system".to_string(),
            content: "A new user has joined".to_string(),
            timestamp: now(),
        });

        // Spawn task to forward broadcasts to this client
        let send_task = tokio::spawn(async move {
            while let Some(result) = broadcast_rx.recv().await {
                match result {
                    Ok(msg) => {
                        if let Err(e) = sender.send(msg).await {
                            tracing::debug!("Send error: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Broadcast error: {}", e);
                    }
                }
            }
        });

        // Handle incoming messages
        while let Some(result) = receiver.recv().await {
            match result {
                Ok(msg) => {
                    if let Some(text) = msg.as_text() {
                        // Broadcast to all clients
                        if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(text) {
                            broadcast.send(Message::text(text.to_string()));
                            tracing::info!("[{}] {}", chat_msg.username, chat_msg.content);
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Receive error: {}", e);
                    break;
                }
            }
        }

        // Clean up
        send_task.abort();

        // Announce user left
        let _ = broadcast.send_json(&ChatMessage {
            username: "system".to_string(),
            content: "A user has left".to_string(),
            timestamp: now(),
        });

        tracing::info!(
            "Chat connection closed (remaining: {})",
            broadcast.subscriber_count()
        );
    })
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Index page with WebSocket test client
async fn index() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>WebSocket Example</title>
    <style>
        body { font-family: sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }
        .section { margin: 30px 0; padding: 20px; border: 1px solid #ddd; border-radius: 8px; }
        h2 { margin-top: 0; }
        input, button { padding: 10px; margin: 5px; }
        #messages { height: 200px; overflow-y: auto; border: 1px solid #ccc; padding: 10px; background: #f5f5f5; }
        .msg { padding: 5px 0; border-bottom: 1px solid #eee; }
        .system { color: #666; font-style: italic; }
    </style>
</head>
<body>
    <h1>🔌 WebSocket Example</h1>
    
    <div class="section">
        <h2>Echo Test (/ws/echo)</h2>
        <input type="text" id="echoInput" placeholder="Type a message...">
        <button onclick="sendEcho()">Send</button>
        <button onclick="connectEcho()">Connect</button>
        <button onclick="disconnectEcho()">Disconnect</button>
        <div id="echoMessages"></div>
    </div>
    
    <div class="section">
        <h2>Chat Room (/ws/chat)</h2>
        <input type="text" id="username" placeholder="Username" value="user1">
        <input type="text" id="chatInput" placeholder="Type a message...">
        <button onclick="sendChat()">Send</button>
        <button onclick="connectChat()">Connect</button>
        <button onclick="disconnectChat()">Disconnect</button>
        <div id="chatMessages"></div>
    </div>
    
    <script>
        let echoWs = null;
        let chatWs = null;
        
        function connectEcho() {
            echoWs = new WebSocket('ws://localhost:8080/ws/echo');
            echoWs.onmessage = (e) => {
                addMessage('echoMessages', e.data);
            };
            echoWs.onopen = () => addMessage('echoMessages', '✓ Connected');
            echoWs.onclose = () => addMessage('echoMessages', '✗ Disconnected');
        }
        
        function disconnectEcho() {
            if (echoWs) echoWs.close();
        }
        
        function sendEcho() {
            if (echoWs && echoWs.readyState === WebSocket.OPEN) {
                const msg = document.getElementById('echoInput').value;
                echoWs.send(msg);
                addMessage('echoMessages', '→ ' + msg, 'sent');
                document.getElementById('echoInput').value = '';
            }
        }
        
        function connectChat() {
            chatWs = new WebSocket('ws://localhost:8080/ws/chat');
            chatWs.onmessage = (e) => {
                try {
                    const msg = JSON.parse(e.data);
                    const cls = msg.username === 'system' ? 'system' : '';
                    addMessage('chatMessages', `[${msg.username}] ${msg.content}`, cls);
                } catch {
                    addMessage('chatMessages', e.data);
                }
            };
            chatWs.onopen = () => addMessage('chatMessages', '✓ Connected to chat', 'system');
            chatWs.onclose = () => addMessage('chatMessages', '✗ Disconnected from chat', 'system');
        }
        
        function disconnectChat() {
            if (chatWs) chatWs.close();
        }
        
        function sendChat() {
            if (chatWs && chatWs.readyState === WebSocket.OPEN) {
                const username = document.getElementById('username').value || 'anonymous';
                const content = document.getElementById('chatInput').value;
                chatWs.send(JSON.stringify({ username, content, timestamp: Date.now() / 1000 }));
                document.getElementById('chatInput').value = '';
            }
        }
        
        function addMessage(containerId, text, className = '') {
            const container = document.getElementById(containerId);
            const div = document.createElement('div');
            div.className = 'msg ' + className;
            div.textContent = text;
            container.appendChild(div);
            container.scrollTop = container.scrollHeight;
        }
        
        // Auto-connect on load
        setTimeout(() => {
            connectEcho();
            connectChat();
        }, 100);
    </script>
</body>
</html>"#,
    )
}

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rustapi_ws=debug".parse().unwrap())
                .add_directive("websocket_example=debug".parse().unwrap())
                .add_directive("info".parse().unwrap()),
        )
        .init();

    let state = Arc::new(AppState {
        chat_broadcast: Arc::new(Broadcast::new()),
    });

    let addr = "127.0.0.1:8080";
    tracing::info!("🚀 Server running at http://{}", addr);
    tracing::info!("📡 WebSocket endpoints:");
    tracing::info!("   ws://{}/ws/echo - Echo server", addr);
    tracing::info!("   ws://{}/ws/json - JSON echo", addr);
    tracing::info!("   ws://{}/ws/chat - Chat room", addr);

    RustApi::new()
        .state(state)
        .route("/", get(index))
        .route("/ws/echo", get(ws_echo))
        .route("/ws/json", get(ws_json))
        .route("/ws/chat", get(ws_chat))
        .run(addr)
        .await
}
