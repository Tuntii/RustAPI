// Integration Test Suite for RustAPI
// This file runs high-level flows to ensure "Action" correctness.

use rustapi_rs::prelude::*;
use rustapi_testing::TestClient;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_api_flow() {
        // Define a handler
        async fn hello() -> &'static str {
            "Hello, World!"
        }

        async fn echo(body: String) -> String {
            body
        }

        // Setup app
        let app = RustApi::new()
            .route("/hello", get(hello))
            .route("/echo", post(echo));
        
        // Use TestClient
        let client = TestClient::new(app);

        // Test GET
        client.get("/hello")
            .await
            .assert_status(200)
            .assert_body_contains("Hello, World!");

        // Test POST
        client.post_json("/echo", &"Checking echo".to_string())
            .await
            .assert_status(200)
            .assert_body_contains("Checking echo");
    }



}
