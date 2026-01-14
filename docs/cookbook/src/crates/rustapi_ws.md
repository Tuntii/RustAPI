# rustapi-ws: The Live Wire

**Lens**: "The Live Wire"
**Philosophy**: "Real-time, persistent connections made simple."

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

We recommend an **Actor Model** for WebSocket state.
1. Each connection spawns a new async task (the actor).
2. Use `tokio::sync::broadcast` channels for global events (like chat rooms).
3. Use `mpsc` channels for direct messaging.
