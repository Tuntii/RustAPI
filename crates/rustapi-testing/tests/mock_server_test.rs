use http::{Method, StatusCode};
use rustapi_testing::{MockResponse, MockServer, RequestMatcher};
use serde_json::json;

#[tokio::test]
async fn test_mock_server_basics() {
    let server = MockServer::start().await;

    server
        .expect(RequestMatcher::new().method(Method::GET).path("/hello"))
        .respond_with(MockResponse::new().body("Hello World"));

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/hello", server.base_url()))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let text = resp.text().await.unwrap();
    assert_eq!(text, "Hello World");

    server.verify();
}

#[tokio::test]
async fn test_json_matching() {
    let server = MockServer::start().await;

    server
        .expect(
            RequestMatcher::new()
                .method(Method::POST)
                .path("/users")
                .body_json(json!({"name": "Alice"})),
        )
        .respond_with(MockResponse::new().status(StatusCode::CREATED));

    let client = reqwest::Client::new();

    // Test match
    let resp = client
        .post(format!("{}/users", server.base_url()))
        .json(&json!({"name": "Alice"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Test non-match (wrong body)
    let resp = client
        .post(format!("{}/users", server.base_url()))
        .json(&json!({"name": "Bob"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_times_verification() {
    let server = MockServer::start().await;

    server
        .expect(RequestMatcher::new().path("/once"))
        .respond_with(MockResponse::new())
        .times(2); // Expect 2 calls

    let client = reqwest::Client::new();
    let url = format!("{}/once", server.base_url());

    client.get(&url).send().await.unwrap();
    client.get(&url).send().await.unwrap();

    server.verify(); // Should pass
}

#[tokio::test]
#[should_panic]
async fn test_verification_failure() {
    let server = MockServer::start().await;

    server
        .expect(RequestMatcher::new().path("/must-call"))
        .once(); // Expect 1 call

    // No call made
    server.verify(); // Should panic
}
