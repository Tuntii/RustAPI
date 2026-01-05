//! WebSocket stream implementation

use crate::{Message, WebSocketError};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, Stream, StreamExt,
};
use std::pin::Pin;
use std::task::{Context, Poll};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use tokio_tungstenite::WebSocketStream as TungsteniteStream;

/// Type alias for the upgraded connection
type UpgradedConnection = TungsteniteStream<TokioIo<Upgraded>>;

/// A WebSocket stream that wraps the underlying tungstenite stream
///
/// This provides a simple interface for sending and receiving WebSocket messages.
/// You can either use the stream directly with `send`/`recv` methods, or split
/// it into separate sender and receiver halves for concurrent operations.
#[allow(dead_code)]
pub struct WebSocketStream {
    inner: UpgradedConnection,
}

impl WebSocketStream {
    /// Create a new WebSocket stream from an upgraded connection
    #[allow(dead_code)]
    pub(crate) fn new(inner: UpgradedConnection) -> Self {
        Self { inner }
    }

    /// Split the stream into sender and receiver halves
    ///
    /// This allows concurrent sending and receiving on the same connection.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let (mut sender, mut receiver) = socket.split();
    ///
    /// // Now you can use sender and receiver concurrently
    /// tokio::select! {
    ///     msg = receiver.recv() => { /* handle incoming */ }
    ///     _ = sender.send(Message::text("ping")) => { /* sent */ }
    /// }
    /// ```
    pub fn split(self) -> (WebSocketSender, WebSocketReceiver) {
        let (sink, stream) = self.inner.split();
        (WebSocketSender { inner: sink }, WebSocketReceiver { inner: stream })
    }

    /// Send a message
    pub async fn send(&mut self, msg: Message) -> Result<(), WebSocketError> {
        self.inner
            .send(msg.into())
            .await
            .map_err(WebSocketError::from)
    }

    /// Send a text message
    pub async fn send_text(&mut self, text: impl Into<String>) -> Result<(), WebSocketError> {
        self.send(Message::text(text)).await
    }

    /// Send a binary message
    pub async fn send_binary(&mut self, data: impl Into<Vec<u8>>) -> Result<(), WebSocketError> {
        self.send(Message::binary(data)).await
    }

    /// Send a JSON message
    pub async fn send_json<T: serde::Serialize>(&mut self, value: &T) -> Result<(), WebSocketError> {
        let msg = Message::json(value)?;
        self.send(msg).await
    }

    /// Receive the next message
    pub async fn recv(&mut self) -> Option<Result<Message, WebSocketError>> {
        self.inner
            .next()
            .await
            .map(|result| result.map(Message::from).map_err(WebSocketError::from))
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<(), WebSocketError> {
        self.inner
            .close(None)
            .await
            .map_err(WebSocketError::from)
    }

    /// Close the connection with a close frame
    pub async fn close_with(
        mut self,
        code: crate::CloseCode,
        reason: impl Into<String>,
    ) -> Result<(), WebSocketError> {
        let frame = tungstenite::protocol::CloseFrame {
            code: code.into(),
            reason: reason.into().into(),
        };
        self.inner
            .close(Some(frame))
            .await
            .map_err(WebSocketError::from)
    }
}

/// Sender half of a WebSocket stream
///
/// This is obtained by calling `split()` on a `WebSocketStream`.
pub struct WebSocketSender {
    inner: SplitSink<UpgradedConnection, tungstenite::Message>,
}

impl WebSocketSender {
    /// Send a message
    pub async fn send(&mut self, msg: Message) -> Result<(), WebSocketError> {
        self.inner
            .send(msg.into())
            .await
            .map_err(WebSocketError::from)
    }

    /// Send a text message
    pub async fn send_text(&mut self, text: impl Into<String>) -> Result<(), WebSocketError> {
        self.send(Message::text(text)).await
    }

    /// Send a binary message
    pub async fn send_binary(&mut self, data: impl Into<Vec<u8>>) -> Result<(), WebSocketError> {
        self.send(Message::binary(data)).await
    }

    /// Send a JSON message
    pub async fn send_json<T: serde::Serialize>(&mut self, value: &T) -> Result<(), WebSocketError> {
        let msg = Message::json(value)?;
        self.send(msg).await
    }

    /// Flush any buffered messages
    pub async fn flush(&mut self) -> Result<(), WebSocketError> {
        self.inner.flush().await.map_err(WebSocketError::from)
    }

    /// Close the sender
    pub async fn close(mut self) -> Result<(), WebSocketError> {
        self.inner.close().await.map_err(WebSocketError::from)
    }
}

/// Receiver half of a WebSocket stream
///
/// This is obtained by calling `split()` on a `WebSocketStream`.
pub struct WebSocketReceiver {
    inner: SplitStream<UpgradedConnection>,
}

impl WebSocketReceiver {
    /// Receive the next message
    pub async fn recv(&mut self) -> Option<Result<Message, WebSocketError>> {
        self.next().await
    }

    /// Receive the next text message, skipping non-text messages
    pub async fn recv_text(&mut self) -> Option<Result<String, WebSocketError>> {
        loop {
            match self.recv().await {
                Some(Ok(Message::Text(text))) => return Some(Ok(text)),
                Some(Ok(Message::Close(_))) => return None,
                Some(Err(e)) => return Some(Err(e)),
                Some(Ok(_)) => continue, // Skip non-text messages
                None => return None,
            }
        }
    }

    /// Receive and deserialize a JSON message
    pub async fn recv_json<T: serde::de::DeserializeOwned>(
        &mut self,
    ) -> Option<Result<T, WebSocketError>> {
        match self.recv().await {
            Some(Ok(msg)) => Some(msg.as_json()),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

impl Stream for WebSocketReceiver {
    type Item = Result<Message, WebSocketError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(msg))) => Poll::Ready(Some(Ok(Message::from(msg)))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(WebSocketError::from(e)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
