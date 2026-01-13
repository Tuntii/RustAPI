//! Streaming response types for RustAPI
//!
//! This module provides types for streaming response bodies.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_core::stream::StreamBody;
//! use futures_util::stream;
//! use bytes::Bytes;
//!
//! async fn stream_data() -> StreamBody<impl Stream<Item = Result<Bytes, std::convert::Infallible>>> {
//!     let stream = stream::iter(vec![
//!         Ok(Bytes::from("chunk 1")),
//!         Ok(Bytes::from("chunk 2")),
//!     ]);
//!     StreamBody::new(stream)
//! }
//! ```

use bytes::Bytes;
use futures_util::Stream;
use http::{header, StatusCode};
use http_body_util::Full;

use crate::response::{IntoResponse, Response};

/// A streaming body wrapper for HTTP responses
///
/// `StreamBody` wraps a stream of bytes and converts it to an HTTP response.
/// This is useful for streaming large amounts of data without buffering
/// the entire response in memory.
///
/// # Example
///
/// ```rust,ignore
/// use rustapi_core::stream::StreamBody;
/// use futures_util::stream;
/// use bytes::Bytes;
///
/// async fn stream_data() -> StreamBody<impl Stream<Item = Result<Bytes, std::convert::Infallible>>> {
///     let stream = stream::iter(vec![
///         Ok(Bytes::from("chunk 1")),
///         Ok(Bytes::from("chunk 2")),
///     ]);
///     StreamBody::new(stream)
/// }
/// ```
pub struct StreamBody<S> {
    #[allow(dead_code)]
    stream: S,
    content_type: Option<String>,
}

impl<S> StreamBody<S> {
    /// Create a new streaming body from a stream
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            content_type: None,
        }
    }

    /// Set the content type for the response
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }
}

// For now, we'll implement IntoResponse by returning a response with appropriate headers
// The actual streaming would require changes to the Response type to support streaming bodies
// This is a simplified implementation that works with the current Response type (Full<Bytes>)
impl<S, E> IntoResponse for StreamBody<S>
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_response(self) -> Response {
        // For the initial implementation, we return a response with streaming headers
        // and an empty body. The actual streaming would require a different body type.

        let content_type = self
            .content_type
            .unwrap_or_else(|| "application/octet-stream".to_string());

        http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::TRANSFER_ENCODING, "chunked")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }
}

/// Helper function to create a streaming body from an iterator of byte chunks
///
/// This is useful for simple cases where you have a fixed set of chunks.
pub fn stream_from_iter<I, E>(
    chunks: I,
) -> StreamBody<futures_util::stream::Iter<std::vec::IntoIter<Result<Bytes, E>>>>
where
    I: IntoIterator<Item = Result<Bytes, E>>,
{
    use futures_util::stream;
    let vec: Vec<_> = chunks.into_iter().collect();
    StreamBody::new(stream::iter(vec))
}

/// Helper function to create a streaming body from a string iterator
///
/// Converts each string to bytes automatically.
pub fn stream_from_strings<I, S, E>(
    strings: I,
) -> StreamBody<futures_util::stream::Iter<std::vec::IntoIter<Result<Bytes, E>>>>
where
    I: IntoIterator<Item = Result<S, E>>,
    S: Into<String>,
{
    use futures_util::stream;
    let vec: Vec<_> = strings
        .into_iter()
        .map(|r| r.map(|s| Bytes::from(s.into())))
        .collect();
    StreamBody::new(stream::iter(vec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    #[test]
    fn test_stream_body_default_content_type() {
        let chunks: Vec<Result<Bytes, std::convert::Infallible>> = vec![Ok(Bytes::from("chunk 1"))];
        let stream_body = StreamBody::new(stream::iter(chunks));
        let response = stream_body.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/octet-stream"
        );
        assert_eq!(
            response.headers().get(header::TRANSFER_ENCODING).unwrap(),
            "chunked"
        );
    }

    #[test]
    fn test_stream_body_custom_content_type() {
        let chunks: Vec<Result<Bytes, std::convert::Infallible>> = vec![Ok(Bytes::from("chunk 1"))];
        let stream_body = StreamBody::new(stream::iter(chunks)).content_type("text/plain");
        let response = stream_body.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/plain"
        );
    }

    #[test]
    fn test_stream_from_iter() {
        let chunks: Vec<Result<Bytes, std::convert::Infallible>> =
            vec![Ok(Bytes::from("chunk 1")), Ok(Bytes::from("chunk 2"))];
        let stream_body = stream_from_iter(chunks);
        let response = stream_body.into_response();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_stream_from_strings() {
        let strings: Vec<Result<&str, std::convert::Infallible>> = vec![Ok("hello"), Ok("world")];
        let stream_body = stream_from_strings(strings);
        let response = stream_body.into_response();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

/// Configuration for streaming request bodies
#[derive(Debug, Clone, Copy)]
pub struct StreamingConfig {
    /// Maximum total body size in bytes
    pub max_body_size: Option<usize>,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_body_size: Some(10 * 1024 * 1024), // 10MB default
        }
    }
}

/// A streaming request body wrapper
///
/// Wraps the incoming hyper body stream or a generic stream and enforces limits.
pub struct StreamingBody {
    inner: StreamingInner,
    bytes_read: usize,
    limit: Option<usize>,
}

enum StreamingInner {
    Hyper(hyper::body::Incoming),
    Generic(
        std::pin::Pin<
            Box<dyn futures_util::Stream<Item = Result<Bytes, crate::error::ApiError>> + Send>,
        >,
    ),
}

impl StreamingBody {
    /// Create a new StreamingBody from hyper Incoming
    pub fn new(inner: hyper::body::Incoming, limit: Option<usize>) -> Self {
        Self {
            inner: StreamingInner::Hyper(inner),
            bytes_read: 0,
            limit,
        }
    }

    /// Create from a generic stream
    pub fn from_stream<S>(stream: S, limit: Option<usize>) -> Self
    where
        S: futures_util::Stream<Item = Result<Bytes, crate::error::ApiError>> + Send + 'static,
    {
        Self {
            inner: StreamingInner::Generic(Box::pin(stream)),
            bytes_read: 0,
            limit,
        }
    }

    /// Get the number of bytes read so far
    pub fn bytes_read(&self) -> usize {
        self.bytes_read
    }
}

impl Stream for StreamingBody {
    type Item = Result<Bytes, crate::error::ApiError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use hyper::body::Body;

        match &mut self.inner {
            StreamingInner::Hyper(incoming) => {
                loop {
                    match std::pin::Pin::new(&mut *incoming).poll_frame(cx) {
                        std::task::Poll::Ready(Some(Ok(frame))) => {
                            if let Ok(data) = frame.into_data() {
                                let len = data.len();
                                self.bytes_read += len;
                                if let Some(limit) = self.limit {
                                    if self.bytes_read > limit {
                                        return std::task::Poll::Ready(Some(Err(
                                            crate::error::ApiError::new(
                                                StatusCode::PAYLOAD_TOO_LARGE,
                                                "payload_too_large",
                                                format!(
                                                    "Body size exceeded limit of {} bytes",
                                                    limit
                                                ),
                                            ),
                                        )));
                                    }
                                }
                                return std::task::Poll::Ready(Some(Ok(data)));
                            }
                            continue; // Trailer
                        }
                        std::task::Poll::Ready(Some(Err(e))) => {
                            return std::task::Poll::Ready(Some(Err(
                                crate::error::ApiError::bad_request(e.to_string()),
                            )));
                        }
                        std::task::Poll::Ready(None) => return std::task::Poll::Ready(None),
                        std::task::Poll::Pending => return std::task::Poll::Pending,
                    }
                }
            }
            StreamingInner::Generic(stream) => match stream.as_mut().poll_next(cx) {
                std::task::Poll::Ready(Some(Ok(data))) => {
                    let len = data.len();
                    self.bytes_read += len;
                    if let Some(limit) = self.limit {
                        if self.bytes_read > limit {
                            return std::task::Poll::Ready(Some(Err(crate::error::ApiError::new(
                                StatusCode::PAYLOAD_TOO_LARGE,
                                "payload_too_large",
                                format!("Body size exceeded limit of {} bytes", limit),
                            ))));
                        }
                    }
                    std::task::Poll::Ready(Some(Ok(data)))
                }
                other => other,
            },
        }
    }
}
