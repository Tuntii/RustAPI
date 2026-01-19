use futures_util::StreamExt;
use http::StatusCode;
use proptest::prelude::*;
use rustapi_core::post;
use rustapi_core::BodyStream;
use rustapi_core::RustApi;
use rustapi_testing::{TestClient, TestRequest};

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

    // TestClient::with_body_limit can set larger limit for the middleware layer
    let client = TestClient::with_body_limit(app, body_len + 1024);

    // Now BodyLimitLayer should pass it.
    // But StreamingBody (inside handler) has hardcoded default 10MB limit.
    // So StreamingBody should fail.

    let response = client
        .request(TestRequest::post("/stream").body(bytes))
        .await;

    // Handler catches error and returns string "Error: ..."
    response.assert_status(StatusCode::OK);
    response.assert_body_contains("payload_too_large");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))] // Fewer cases as these are async/heavy

    #[test]
    fn prop_streaming_body_limits(
        // Vary body size around the 10MB limit (10 * 1024 * 1024)
        // We test small sizes, near limit, and over limit
        // Using smaller limits for property test efficiency?
        // But StreamingBody defaults to 10MB.
        // Let's rely on logic correctness and test:
        // 1. Small bodies pass
        // 2. We can't easily change StreamingBody default limit without Config injection (TODO).
        // So we test with smaller static limit if possible or just standard 10MB is too large for 100 iterations.

        // Actually, we can just test that *any* body under 10MB passes correctly.
        body_len in 0usize..100_000usize
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let body = vec![0u8; body_len];
            let bytes = bytes::Bytes::from(body.clone());

            async fn handler(mut stream: BodyStream) -> String {
                let mut size = 0;
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(b) => size += b.len(),
                        Err(e) => return format!("Error: {}", e),
                    }
                }
                format!("Size: {}", size)
            }

            let app = RustApi::new().route("/stream", post(handler));
            let client = TestClient::new(app); // Default limit 1MB for BodyLimitLayer... wait.

            // BodyLimitLayer defaults to 1MB (1024*1024).
            // Our test body is up to 100KB, so it passes BodyLimitLayer.
            // StreamingBody default is 10MB.

            // So this should always succeed.

            let response = client
                .request(TestRequest::post("/stream").body(bytes))
                .await;

            response.assert_status(StatusCode::OK);
            assert_eq!(response.text(), format!("Size: {}", body_len));
        });
    }
}
