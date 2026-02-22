# Testing Strategies

RustAPI provides robust tools for testing your application, ensuring reliability from unit tests to full integration scenarios.

## Dependencies

Add `rustapi-testing` to your `Cargo.toml`. It is usually added as a dev-dependency.

```toml
[dev-dependencies]
rustapi-testing = "0.1.335"
tokio = { version = "1", features = ["full"] }
```

## Integration Testing with TestClient

The `TestClient` allows you to test your API handlers without binding to a network port. It interacts directly with the service layer, making tests fast and deterministic.

```rust
use rustapi_rs::prelude::*;
use rustapi_testing::TestClient;

#[rustapi_rs::get("/hello")]
async fn hello() -> &'static str {
    "Hello, World!"
}

#[tokio::test]
async fn test_hello_endpoint() {
    // 1. Build your application
    let app = RustApi::new().route("/hello", get(hello));

    // 2. Create a TestClient
    let client = TestClient::new(app);

    // 3. Send requests
    let response = client.get("/hello").send().await;

    // 4. Assert response
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await, "Hello, World!");
}
```

### Testing JSON APIs

`TestClient` has built-in support for JSON serialization and deserialization.

```rust
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct User {
    id: u64,
    name: String,
}

#[rustapi_rs::post("/users")]
async fn create_user(Json(user): Json<User>) -> Json<User> {
    Json(user)
}

#[tokio::test]
async fn test_create_user() {
    let app = RustApi::new().route("/users", post(create_user));
    let client = TestClient::new(app);

    let new_user = User { id: 1, name: "Alice".into() };

    let response = client.post("/users")
        .json(&new_user)
        .send()
        .await;

    assert_eq!(response.status(), 200);

    let returned_user: User = response.json().await;
    assert_eq!(returned_user, new_user);
}
```

## Mocking External Services

When your API calls external services (e.g., payment gateways, third-party APIs), you should mock them in tests to avoid network calls and ensure reproducibility.

`rustapi-testing` provides `MockServer` for this purpose.

```rust
use rustapi_testing::{MockServer, MockResponse};

#[tokio::test]
async fn test_external_integration() {
    // 1. Start a mock server
    let mock_server = MockServer::start().await;

    // 2. Define an expectation
    mock_server.expect(
        rustapi_testing::RequestMatcher::new()
            .method("GET")
            .path("/external-data")
    ).respond_with(
        MockResponse::new()
            .status(200)
            .body(r#"{"data": "mocked"}"#)
    );

    // 3. Use the mock server's URL in your app configuration
    let mock_url = format!("{}{}", mock_server.base_url(), "/external-data");

    // Simulating your app logic calling the external service
    let client = reqwest::Client::new();
    let res = client.get(&mock_url).send().await.unwrap();

    assert_eq!(res.status(), 200);
    let body = res.text().await.unwrap();
    assert_eq!(body, r#"{"data": "mocked"}"#);
}
```

## Testing Authenticated Routes

You can simulate authenticated requests by setting headers directly on the `TestClient` request builder.

```rust
#[tokio::test]
async fn test_protected_route() {
    let app = RustApi::new().route("/protected", get(protected_handler));
    let client = TestClient::new(app);

    let response = client.get("/protected")
        .header("Authorization", "Bearer valid_token")
        .send()
        .await;

    assert_eq!(response.status(), 200);
}
```

## Best Practices

1.  **Keep Tests Independent**: Each test should setup its own app instance and state. `TestClient` is lightweight enough for this.
2.  **Mock I/O**: Use `MockServer` for HTTP, and in-memory implementations for databases (e.g., `sqlite::memory:`) or traits for logic.
3.  **Test Edge Cases**: Don't just test the "happy path". Test validation errors, 404s, and error handling.
