//! WebSocket stream implementation

use crate::{Message, WebSocketError, WsHeartbeatConfig};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, Stream, StreamExt,
};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream as TungsteniteStream;

/// Type alias for the upgraded connection
type UpgradedConnection = TungsteniteStream<TokioIo<Upgraded>>;

/// Internal implementation of the WebSocket stream
enum StreamImpl {
    /// Direct connection (no heartbeat/management)
    Direct(Box<UpgradedConnection>),
    /// Managed connection (heartbeat/cleanup running in background task)
    Managed {
        tx: mpsc::Sender<Message>,
        rx: mpsc::Receiver<Result<Message, WebSocketError>>,
    },
}

/// A WebSocket stream
pub struct WebSocketStream {
    inner: StreamImpl,
}

impl WebSocketStream {
    /// Create a new direct WebSocket stream
    pub(crate) fn new(inner: UpgradedConnection) -> Self {
        Self {
            inner: StreamImpl::Direct(Box::new(inner)),
        }
    }

    /// Create a new managed WebSocket stream with heartbeat
    pub(crate) fn new_managed(inner: UpgradedConnection, config: WsHeartbeatConfig) -> Self {
        let (mut sender, mut receiver) = inner.split();
        let (user_tx, mut internal_rx) = mpsc::channel::<Message>(32);
        let (internal_tx, user_rx) = mpsc::channel::<Result<Message, WebSocketError>>(32);

        // Spawn management task
        tokio::spawn(async move {
            let mut heartbeat_interval = tokio::time::interval(config.interval);
            // First tick finishes immediately
            heartbeat_interval.tick().await;

            // For pong tracking, we can just track last activity or strictly check pongs.
            // Simplified: we rely on TCP checks and ping writing success.
            // If we want to enforce timeout, we need to track "last pong".

            // Tungstenite handles Pong responses to our Pings automatically IF we poll the stream.
            // But we are polling the stream in the select loop below.

            // Note: Tungstenite returns Pongs as messages. We should filter them out mostly,
            // or pass them if the user wants them?
            // Usually heartbeat Pongs are an implementation detail.

            let mut last_heartbeat = tokio::time::Instant::now();
            let mut timeout_check = tokio::time::interval(config.timeout);

            loop {
                tokio::select! {
                    // 1. Receive message from socket
                    msg = receiver.next() => {
                        match msg {
                            Some(Ok(msg)) => {
                                last_heartbeat = tokio::time::Instant::now();
                                if msg.is_pong() {
                                    // Received a pong (response to our ping)
                                    continue;
                                }
                                if msg.is_ping() {
                                    // Received a ping (from client)
                                    // Tungstenite might have auto-replied if we used the right callback,
                                    // but default poll_next does reply to pings by queueing a pong.
                                    // We need to ensure that queued pong is sent.
                                    // However, we are in a split stream.
                                    // `receiver` is Stream. `sender` is Sink.
                                    // Tungstenite's split separates them.
                                    // The `receiver` will NOT automatically write to `sender`.
                                    // WE must handle Ping replies if split?
                                    // `tokio-tungstenite` docs: "You must handle Pings manually when split?"
                                    // No, the `tungstenite` protocol handler is shared? No.

                                    // If we receive a Ping, we should send a Pong.
                                    let _ = sender.send(Message::Pong(msg.into_data()).into()).await;
                                    continue;
                                }

                                // Forward other messages to user
                                if internal_tx.send(Ok(Message::from(msg))).await.is_err() {
                                    break; // User dropped receiver
                                }
                            }
                            Some(Err(e)) => {
                                let _ = internal_tx.send(Err(WebSocketError::from(e))).await;
                                break;
                            }
                            None => break, // Connection closed
                        }
                    }

                    // 2. Receive message from user to send
                    msg = internal_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                if sender.send(msg.into()).await.is_err() {
                                    break; // Connection closed
                                }
                            }
                            None => break, // User dropped sender
                        }
                    }

                    // 3. Send Ping
                    _ = heartbeat_interval.tick() => {
                         if sender.send(Message::Ping(vec![]).into()).await.is_err() {
                             break;
                         }
                    }

                    // 4. Check timeout
                    _ = timeout_check.tick() => {
                        if last_heartbeat.elapsed() > config.interval + config.timeout {
                            // Timeout
                            break;
                            // This drops 'sender', closing the connection
                        }
                    }
                }
            }
            // Loop break drops sender/receiver, closing connection
        });

        Self {
            inner: StreamImpl::Managed {
                tx: user_tx,
                rx: user_rx,
            },
        }
    }

    /// Split the stream into sender and receiver halves
    pub fn split(self) -> (WebSocketSender, WebSocketReceiver) {
        match self.inner {
            StreamImpl::Direct(inner) => {
                let (sink, stream) = inner.split();
                (
                    WebSocketSender {
                        inner: SenderImpl::Direct(sink),
                    },
                    WebSocketReceiver {
                        inner: ReceiverImpl::Direct(stream),
                    },
                )
            }
            StreamImpl::Managed { tx, rx } => (
                WebSocketSender {
                    inner: SenderImpl::Managed(tx),
                },
                WebSocketReceiver {
                    inner: ReceiverImpl::Managed(rx),
                },
            ),
        }
    }
}

// Implement helper methods directly on WebSocketStream for convenience
impl WebSocketStream {
    /// Send a message
    pub async fn send(&mut self, msg: Message) -> Result<(), WebSocketError> {
        match &mut self.inner {
            StreamImpl::Direct(s) => s.send(msg.into()).await.map_err(WebSocketError::from),
            StreamImpl::Managed { tx, .. } => tx
                .send(msg)
                .await
                .map_err(|_| WebSocketError::ConnectionClosed),
        }
    }

    /// Receive the next message
    pub async fn recv(&mut self) -> Option<Result<Message, WebSocketError>> {
        match &mut self.inner {
            StreamImpl::Direct(s) => s
                .next()
                .await
                .map(|r| r.map(Message::from).map_err(WebSocketError::from)),
            StreamImpl::Managed { rx, .. } => rx.recv().await,
        }
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
    pub async fn send_json<T: serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), WebSocketError> {
        self.send(Message::json(value)?).await
    }
}

// Inner implementations for Sender/Receiver

enum SenderImpl {
    Direct(SplitSink<UpgradedConnection, tungstenite::Message>),
    Managed(mpsc::Sender<Message>),
}

/// Sender half of a WebSocket stream
pub struct WebSocketSender {
    inner: SenderImpl,
}

impl WebSocketSender {
    /// Send a message
    pub async fn send(&mut self, msg: Message) -> Result<(), WebSocketError> {
        match &mut self.inner {
            SenderImpl::Direct(s) => s.send(msg.into()).await.map_err(WebSocketError::from),
            SenderImpl::Managed(s) => s
                .send(msg)
                .await
                .map_err(|_| WebSocketError::ConnectionClosed),
        }
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
    pub async fn send_json<T: serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), WebSocketError> {
        self.send(Message::json(value)?).await
    }

    /// Close the sender
    pub async fn close(mut self) -> Result<(), WebSocketError> {
        match &mut self.inner {
            SenderImpl::Direct(s) => s.close().await.map_err(WebSocketError::from),
            SenderImpl::Managed(_) => {
                // Drop sender to close channel, explicitly nothing else to do
                Ok(())
            }
        }
    }
}

enum ReceiverImpl {
    Direct(SplitStream<UpgradedConnection>),
    Managed(mpsc::Receiver<Result<Message, WebSocketError>>),
}

/// Receiver half of a WebSocket stream
pub struct WebSocketReceiver {
    inner: ReceiverImpl,
}

impl WebSocketReceiver {
    /// Receive the next message
    pub async fn recv(&mut self) -> Option<Result<Message, WebSocketError>> {
        match &mut self.inner {
            ReceiverImpl::Direct(s) => s
                .next()
                .await
                .map(|r| r.map(Message::from).map_err(WebSocketError::from)),
            ReceiverImpl::Managed(s) => s.recv().await,
        }
    }
}

impl Stream for WebSocketReceiver {
    type Item = Result<Message, WebSocketError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.inner {
            ReceiverImpl::Direct(s) => match Pin::new(s).poll_next(cx) {
                Poll::Ready(Some(Ok(msg))) => Poll::Ready(Some(Ok(Message::from(msg)))),
                Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(WebSocketError::from(e)))),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            },
            ReceiverImpl::Managed(s) => s.poll_recv(cx),
        }
    }
}
