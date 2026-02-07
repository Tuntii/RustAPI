# rustapi-testing: The Auditor

**Lens**: "The Auditor"
**Philosophy**: "Trust, but verify."

`rustapi-testing` provides a comprehensive suite of tools for integration testing your RustAPI applications. It focuses on two main areas:
1. **In-process API testing**: Testing your endpoints without binding to a real TCP port.
2. **External service mocking**: Mocking downstream services (like payment gateways or auth providers) that your API calls.

## The `TestClient`

Integration testing is often slow and painful because it involves spinning up a server, waiting for ports, and managing child processes. `TestClient` solves this by wrapping your `RustApi` application and executing requests directly against the service layer.

### Basic Usage

```rust,ignore
use rustapi_rs::prelude::*;
use rustapi_testing::TestClient;

#[tokio::test]
async fn test_hello_world() {
    let app = RustApi::new().route("/", get(|| async { "Hello!" }));
    let client = TestClient::new(app);

    let response = client.get("/").await;

    response
        .assert_status(200)
        .assert_body_contains("Hello!");
}
```

### Testing JSON APIs

The client provides fluent helpers for JSON APIs.

```rust,ignore
#[derive(Serialize)]
struct CreateUser {
    username: String,
}

#[tokio::test]
async fn test_create_user() {
    let app = RustApi::new().route("/users", post(create_user_handler));
    let client = TestClient::new(app);

    let response = client.post_json("/users", &CreateUser {
        username: "alice".into()
    }).await;

    response
        .assert_status(201)
        .assert_json(&serde_json::json!({
            "id": 1,
            "username": "alice"
        }));
}
```

## Mocking Services with `MockServer`

Real-world applications usually talk to other services. `MockServer` allows you to spin up a lightweight HTTP server that responds to requests based on pre-defined expectations.

### Setting up a Mock Server

```rust,ignore
use rustapi_testing::{MockServer, MockResponse, RequestMatcher};

#[tokio::test]
async fn test_external_integration() {
    // 1. Start the mock server
    let server = MockServer::start().await;

    // 2. Define an expectation
    server.expect(RequestMatcher::new(Method::GET, "/external-api/data"))
        .respond_with(MockResponse::new()
            .status(StatusCode::OK)
            .json(serde_json::json!({ "result": "success" })))
        .times(1);

    // 3. Configure your app to use the mock server's URL
    let app = create_app_with_config(Config {
        external_api_url: server.base_url(),
    });

    let client = TestClient::new(app);

    // 4. Run your test
    client.get("/my-endpoint-calling-external").await.assert_status(200);
}
```

### Expectations

You can define strict expectations on how your application interacts with the mock server.

#### Matching Requests

`RequestMatcher` allows matching by method, path, headers, and body.

```rust,ignore
// Match a POST request with specific body
server.expect(RequestMatcher::new(Method::POST, "/webhook")
    .body_contains("event_type=payment_success"))
    .respond_with(MockResponse::new().status(200));
```

#### Verification

The `MockServer` automatically verifies that all expectations were met when it is dropped (at the end of the test scope). If an expectation was set to be called `once` but was never called, the test will panic.

- `.once()`: Must be called exactly once (default).
- `.times(n)`: Must be called exactly `n` times.
- `.at_least_once()`: Must be called 1 or more times.
- `.never()`: Must not be called.

```rust,ignore
// Ensure we don't call the billing API if validation fails
server.expect(RequestMatcher::new(Method::POST, "/charge"))
    .never();
```

## Best Practices

1. **Dependency Injection**: Design your application `State` to accept base URLs for external services so you can inject the `MockServer` URL during tests.
2. **Isolation**: Create a new `MockServer` for each test case to ensure no shared state or interference.
3. **Fluent Assertions**: Use the chainable assertion methods on `TestResponse` to keep tests readable.
