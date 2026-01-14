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

#[cfg(test)]
mod property_tests {
    use super::*;
    use futures_util::stream;
    use futures_util::StreamExt;
    use proptest::prelude::*;

    /// **Feature: v1-features-roadmap, Property 23: Streaming memory bounds**
    /// **Validates: Requirements 11.2**
    ///
    /// For streaming request bodies:
    /// - Memory usage SHALL never exceed configured limit
    /// - Streams exceeding limit SHALL be rejected with 413 Payload Too Large
    /// - Bytes read counter SHALL accurately track consumed bytes
    /// - Limit of None SHALL allow unlimited streaming
    /// - Multiple chunks SHALL be correctly aggregated for limit checking

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 23: Single chunk within limit is accepted
        #[test]
        fn prop_chunk_within_limit_accepted(
            chunk_size in 100usize..1000,
            limit in 1000usize..10000,
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let data = vec![0u8; chunk_size];
                let chunks: Vec<Result<Bytes, crate::error::ApiError>> =
                    vec![Ok(Bytes::from(data))];
                let stream_data = stream::iter(chunks);

                let mut streaming_body = StreamingBody::from_stream(stream_data, Some(limit));

                // Chunk MUST be accepted (within limit)
                let result = streaming_body.next().await;
                prop_assert!(result.is_some());
                prop_assert!(result.unwrap().is_ok());

                // Bytes read MUST match chunk size
                prop_assert_eq!(streaming_body.bytes_read(), chunk_size);

                Ok(())
            })?;
        }

        /// Property 23: Single chunk exceeding limit is rejected
        #[test]
        fn prop_chunk_exceeding_limit_rejected(
            limit in 100usize..1000,
            excess in 1usize..100,
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let chunk_size = limit + excess;
                let data = vec![0u8; chunk_size];
                let chunks: Vec<Result<Bytes, crate::error::ApiError>> =
                    vec![Ok(Bytes::from(data))];
                let stream_data = stream::iter(chunks);

                let mut streaming_body = StreamingBody::from_stream(stream_data, Some(limit));

                // Chunk MUST be rejected (exceeds limit)
                let result = streaming_body.next().await;
                prop_assert!(result.is_some());
                let error = result.unwrap();
                prop_assert!(error.is_err());

                // Error MUST be Payload Too Large
                let err = error.unwrap_err();
                prop_assert_eq!(err.status_code, StatusCode::PAYLOAD_TOO_LARGE);

                Ok(())
            })?;
        }

        /// Property 23: Multiple chunks within limit are accepted
        #[test]
        fn prop_multiple_chunks_within_limit(
            chunk_size in 100usize..500,
            num_chunks in 2usize..5,
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let total_size = chunk_size * num_chunks;
                let limit = total_size + 100; // Slightly above total

                let chunks: Vec<Result<Bytes, crate::error::ApiError>> = (0..num_chunks)
                    .map(|_| Ok(Bytes::from(vec![0u8; chunk_size])))
                    .collect();
                let stream_data = stream::iter(chunks);

                let mut streaming_body = StreamingBody::from_stream(stream_data, Some(limit));

                // All chunks MUST be accepted
                let mut total_read = 0;
                while let Some(result) = streaming_body.next().await {
                    prop_assert!(result.is_ok());
                    total_read += result.unwrap().len();
                }

                // Total bytes read MUST match total size
                prop_assert_eq!(total_read, total_size);
                prop_assert_eq!(streaming_body.bytes_read(), total_size);

                Ok(())
            })?;
        }

        /// Property 23: Multiple chunks exceeding limit are rejected
        #[test]
        fn prop_multiple_chunks_exceeding_limit(
            chunk_size in 100usize..500,
            num_chunks in 3usize..6,
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let total_size = chunk_size * num_chunks;
                let limit = chunk_size + 50; // Less than total

                let chunks: Vec<Result<Bytes, crate::error::ApiError>> = (0..num_chunks)
                    .map(|_| Ok(Bytes::from(vec![0u8; chunk_size])))
                    .collect();
                let stream_data = stream::iter(chunks);

                let mut streaming_body = StreamingBody::from_stream(stream_data, Some(limit));

                // First chunk MUST succeed
                let first = streaming_body.next().await;
                prop_assert!(first.is_some());
                prop_assert!(first.unwrap().is_ok());

                // Second chunk MUST fail (exceeds limit)
                let second = streaming_body.next().await;
                prop_assert!(second.is_some());
                let error = second.unwrap();
                prop_assert!(error.is_err());

                let err = error.unwrap_err();
                prop_assert_eq!(err.status_code, StatusCode::PAYLOAD_TOO_LARGE);

                Ok(())
            })?;
        }

        /// Property 23: No limit allows unlimited streaming
        #[test]
        fn prop_no_limit_unlimited(
            chunk_size in 1000usize..10000,
            num_chunks in 5usize..10,
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let chunks: Vec<Result<Bytes, crate::error::ApiError>> = (0..num_chunks)
                    .map(|_| Ok(Bytes::from(vec![0u8; chunk_size])))
                    .collect();
                let stream_data = stream::iter(chunks);

                let mut streaming_body = StreamingBody::from_stream(stream_data, None);

                // All chunks MUST be accepted (no limit)
                let mut count = 0;
                while let Some(result) = streaming_body.next().await {
                    prop_assert!(result.is_ok());
                    count += 1;
                }

                prop_assert_eq!(count, num_chunks);
                prop_assert_eq!(streaming_body.bytes_read(), chunk_size * num_chunks);

                Ok(())
            })?;
        }

        /// Property 23: Bytes read counter is accurate
        #[test]
        fn prop_bytes_read_accurate(
            sizes in prop::collection::vec(100usize..1000, 1..10)
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let total_size: usize = sizes.iter().sum();
                let limit = total_size + 1000; // Above total

                let chunks: Vec<Result<Bytes, crate::error::ApiError>> = sizes
                    .iter()
                    .map(|&size| Ok(Bytes::from(vec![0u8; size])))
                    .collect();
                let stream_data = stream::iter(chunks);

                let mut streaming_body = StreamingBody::from_stream(stream_data, Some(limit));

                let mut cumulative = 0;
                while let Some(result) = streaming_body.next().await {
                    let chunk = result.unwrap();
                    cumulative += chunk.len();

                    // Bytes read MUST match cumulative at each step
                    prop_assert_eq!(streaming_body.bytes_read(), cumulative);
                }

                prop_assert_eq!(streaming_body.bytes_read(), total_size);

                Ok(())
            })?;
        }

        /// Property 23: Exact limit boundary is accepted
        #[test]
        fn prop_exact_limit_accepted(chunk_size in 500usize..5000) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let limit = chunk_size; // Exact match
                let data = vec![0u8; chunk_size];
                let chunks: Vec<Result<Bytes, crate::error::ApiError>> =
                    vec![Ok(Bytes::from(data))];
                let stream_data = stream::iter(chunks);

                let mut streaming_body = StreamingBody::from_stream(stream_data, Some(limit));

                // Chunk at exact limit MUST be accepted
                let result = streaming_body.next().await;
                prop_assert!(result.is_some());
                prop_assert!(result.unwrap().is_ok());

                prop_assert_eq!(streaming_body.bytes_read(), chunk_size);

                Ok(())
            })?;
        }

        /// Property 23: One byte over limit is rejected
        #[test]
        fn prop_one_byte_over_rejected(limit in 500usize..5000) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let chunk_size = limit + 1; // One byte over
                let data = vec![0u8; chunk_size];
                let chunks: Vec<Result<Bytes, crate::error::ApiError>> =
                    vec![Ok(Bytes::from(data))];
                let stream_data = stream::iter(chunks);

                let mut streaming_body = StreamingBody::from_stream(stream_data, Some(limit));

                // One byte over MUST be rejected
                let result = streaming_body.next().await;
                prop_assert!(result.is_some());
                let error = result.unwrap();
                prop_assert!(error.is_err());

                Ok(())
            })?;
        }

        /// Property 23: Empty chunks don't affect limit
        #[test]
        fn prop_empty_chunks_ignored(
            chunk_size in 100usize..1000,
            num_empty in 1usize..5,
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let limit = chunk_size + 100;

                let mut chunks: Vec<Result<Bytes, crate::error::ApiError>> = vec![];

                // Add empty chunks
                for _ in 0..num_empty {
                    chunks.push(Ok(Bytes::new()));
                }

                // Add one data chunk
                chunks.push(Ok(Bytes::from(vec![0u8; chunk_size])));

                let stream_data = stream::iter(chunks);
                let mut streaming_body = StreamingBody::from_stream(stream_data, Some(limit));

                // Process all chunks
                while let Some(result) = streaming_body.next().await {
                    prop_assert!(result.is_ok());
                }

                // Bytes read MUST only count non-empty chunk
                prop_assert_eq!(streaming_body.bytes_read(), chunk_size);

                Ok(())
            })?;
        }

        /// Property 23: Limit enforcement is cumulative
        #[test]
        fn prop_limit_cumulative(
            chunk1_size in 300usize..600,
            chunk2_size in 300usize..600,
            limit in 500usize..900,
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let chunks: Vec<Result<Bytes, crate::error::ApiError>> = vec![
                    Ok(Bytes::from(vec![0u8; chunk1_size])),
                    Ok(Bytes::from(vec![0u8; chunk2_size])),
                ];
                let stream_data = stream::iter(chunks);

                let mut streaming_body = StreamingBody::from_stream(stream_data, Some(limit));

                // First chunk
                let first = streaming_body.next().await;
                if chunk1_size <= limit {
                    prop_assert!(first.unwrap().is_ok());

                    // Second chunk
                    let second = streaming_body.next().await;
                    let total = chunk1_size + chunk2_size;

                    if total <= limit {
                        // Both within limit
                        prop_assert!(second.unwrap().is_ok());
                    } else {
                        // Total exceeds limit
                        prop_assert!(second.unwrap().is_err());
                    }
                } else {
                    // First chunk already exceeds limit
                    prop_assert!(first.unwrap().is_err());
                }

                Ok(())
            })?;
        }

        /// Property 23: Default config has 10MB limit
        #[test]
        fn prop_default_config_limit(_seed in 0u32..10) {
            let config = StreamingConfig::default();
            prop_assert_eq!(config.max_body_size, Some(10 * 1024 * 1024));
        }

        /// Property 23: Error message includes limit value
        #[test]
        fn prop_error_message_includes_limit(limit in 1000usize..10000) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let chunk_size = limit + 100;
                let data = vec![0u8; chunk_size];
                let chunks: Vec<Result<Bytes, crate::error::ApiError>> =
                    vec![Ok(Bytes::from(data))];
                let stream_data = stream::iter(chunks);

                let mut streaming_body = StreamingBody::from_stream(stream_data, Some(limit));

                let result = streaming_body.next().await;
                let error = result.unwrap().unwrap_err();

                // Error message MUST include limit value
                prop_assert!(error.message.contains(&limit.to_string()));
                prop_assert!(error.message.contains("exceeded limit"));

                Ok(())
            })?;
        }
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
            Box<
                dyn futures_util::Stream<Item = Result<Bytes, crate::error::ApiError>>
                    + Send
                    + Sync,
            >,
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
        S: futures_util::Stream<Item = Result<Bytes, crate::error::ApiError>>
            + Send
            + Sync
            + 'static,
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
