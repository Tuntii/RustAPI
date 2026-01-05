//! Broadcast channel for WebSocket messages

use crate::Message;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// A broadcast channel for sending messages to multiple WebSocket clients
///
/// This is useful for implementing pub/sub patterns, chat rooms, or any
/// scenario where you need to send the same message to multiple clients.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_ws::{Broadcast, Message};
/// use std::sync::Arc;
///
/// let broadcast = Arc::new(Broadcast::new());
///
/// // Subscribe to receive messages
/// let mut rx = broadcast.subscribe();
///
/// // Send a message to all subscribers
/// broadcast.send(Message::text("Hello everyone!"));
///
/// // Receive the message
/// let msg = rx.recv().await.unwrap();
/// ```
#[derive(Clone)]
pub struct Broadcast {
    sender: broadcast::Sender<Message>,
    subscriber_count: Arc<AtomicUsize>,
}

impl Broadcast {
    /// Create a new broadcast channel with default capacity (100 messages)
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    /// Create a new broadcast channel with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            subscriber_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Subscribe to receive broadcast messages
    pub fn subscribe(&self) -> BroadcastReceiver {
        self.subscriber_count.fetch_add(1, Ordering::SeqCst);
        BroadcastReceiver {
            inner: self.sender.subscribe(),
            subscriber_count: self.subscriber_count.clone(),
        }
    }

    /// Send a message to all subscribers
    ///
    /// Returns the number of receivers that received the message.
    /// Returns 0 if there are no active subscribers.
    pub fn send(&self, msg: Message) -> usize {
        self.sender.send(msg).unwrap_or(0)
    }

    /// Send a text message to all subscribers
    pub fn send_text(&self, text: impl Into<String>) -> usize {
        self.send(Message::text(text))
    }

    /// Send a JSON message to all subscribers
    pub fn send_json<T: serde::Serialize>(
        &self,
        value: &T,
    ) -> Result<usize, crate::WebSocketError> {
        let msg = Message::json(value)?;
        Ok(self.send(msg))
    }

    /// Get the current number of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.subscriber_count.load(Ordering::SeqCst)
    }

    /// Check if there are any active subscribers
    pub fn has_subscribers(&self) -> bool {
        self.subscriber_count() > 0
    }
}

impl Default for Broadcast {
    fn default() -> Self {
        Self::new()
    }
}

/// Receiver for broadcast messages
pub struct BroadcastReceiver {
    inner: broadcast::Receiver<Message>,
    subscriber_count: Arc<AtomicUsize>,
}

impl BroadcastReceiver {
    /// Receive the next broadcast message
    ///
    /// Returns `None` if the broadcast channel is closed.
    /// Returns `Err` if messages were missed due to slow consumption.
    pub async fn recv(&mut self) -> Option<Result<Message, BroadcastRecvError>> {
        match self.inner.recv().await {
            Ok(msg) => Some(Ok(msg)),
            Err(broadcast::error::RecvError::Closed) => None,
            Err(broadcast::error::RecvError::Lagged(count)) => {
                Some(Err(BroadcastRecvError::Lagged(count)))
            }
        }
    }

    /// Try to receive a message without waiting
    pub fn try_recv(&mut self) -> Option<Result<Message, BroadcastRecvError>> {
        match self.inner.try_recv() {
            Ok(msg) => Some(Ok(msg)),
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(broadcast::error::TryRecvError::Closed) => None,
            Err(broadcast::error::TryRecvError::Lagged(count)) => {
                Some(Err(BroadcastRecvError::Lagged(count)))
            }
        }
    }
}

impl Drop for BroadcastReceiver {
    fn drop(&mut self) {
        self.subscriber_count.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Error when receiving broadcast messages
#[derive(Debug, Clone, Copy)]
pub enum BroadcastRecvError {
    /// Some messages were missed because the receiver is too slow
    Lagged(u64),
}

impl std::fmt::Display for BroadcastRecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lagged(count) => write!(f, "Lagged behind by {} messages", count),
        }
    }
}

impl std::error::Error for BroadcastRecvError {}
