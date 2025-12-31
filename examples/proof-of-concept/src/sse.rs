//! Server-Sent Events (SSE) broadcaster for real-time notifications

use tokio::sync::broadcast;

use crate::models::BookmarkEvent;

/// SSE broadcaster using tokio broadcast channel
pub struct SseBroadcaster {
    sender: broadcast::Sender<BookmarkEvent>,
}

impl SseBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }

    /// Broadcast an event to all connected clients
    pub fn broadcast(&self, event: BookmarkEvent) {
        // Ignore send errors (no receivers)
        let _ = self.sender.send(event);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<BookmarkEvent> {
        self.sender.subscribe()
    }
}

impl Default for SseBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
