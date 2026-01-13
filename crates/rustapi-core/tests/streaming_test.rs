use futures_util::StreamExt;
use http::StatusCode;
use rustapi_core::extract::BodyStream;
use rustapi_core::router::post;
use rustapi_core::test_client::TestClient;
use rustapi_core::RustApi;

#[tokio::test]
async fn test_streaming_body_buffered_small() {
    async fn handler(mut stream: BodyStream) -> String {
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            bytes.extend_from_slice(&chunk.unwrap());
        }
        String::from_utf8(bytes).unwrap()
    }

    let app = RustApi::new().route("/stream", post(handler));
    let client = TestClient::new(app);

    let body = "Hello Streaming World";
    let response = client.post_json("/stream", &body).await;

    // "Hello Streaming World" (JSON encoded string) -> "\"Hello Streaming World\""
    // Wait, post_json serializes string as JSON string?
    // "Hello Streaming World" -> "\"Hello Streaming World\"".

    // TestClient::post_json takes reference.
    // serde_json::to_vec(&"str") -> "\"str\"".

    response.assert_status(StatusCode::OK);
    // output should be exactly input json bytes
    let output = response.text();
    assert_eq!(output, "\"Hello Streaming World\"");
}

#[tokio::test]
async fn test_streaming_body_buffered_large_fail() {
    // Default limit is 10MB (10 * 1024 * 1024).
    // We create a body slightly larger.
    let limit = 10 * 1024 * 1024;
    let body_len = limit + 100;

    // We can't allocate 10MB+ string easily in stack, heap is fine.
    let body = vec![b'a'; body_len];
    let bytes = bytes::Bytes::from(body);

    async fn handler(mut stream: BodyStream) -> String {
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(_) => {}
                Err(e) => return format!("Error: {}", e),
            }
        }
        "Success".to_string()
    }

    let app = RustApi::new().route("/stream", post(handler));

    // We need to bypass BodyLimitLayer in TestClient?
    // TestClient adds BodyLimitLayer default (also 10MB?).
    // If BodyLimitLayer rejects it first, we test generic body limit, not StreamingBody limit.

    // BodyLimitLayer default is DEFAULT_BODY_LIMIT.
    // Let's see what DEFAULT_BODY_LIMIT is.
    // Likely 2MB or similar?

    // If BodyLimitLayer rejects it, it returns 413 Payload Too Large.
    // That verifies memory bounds too.

    // But we want to test StreamingBody bounds specifically.
    // StreamingBody bounds apply even if BodyLimitLayer was bypassed (e.g. infinite limit).

    // TestClient::with_body_limit can set larger limit.
    let client = TestClient::with_body_limit(app, body_len + 1024);

    // Now BodyLimitLayer should pass it.
    // But StreamingBody (inside handler) has hardcoded default 10MB limit.
    // So StreamingBody should fail.

    let response = client
        .request(rustapi_core::test_client::TestRequest::post("/stream").body(bytes))
        .await;

    // Handler catches error and returns string "Error: ..."
    response.assert_status(StatusCode::OK);
    response.assert_body_contains("payload_too_large");
}
