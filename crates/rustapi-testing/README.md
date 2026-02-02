# rustapi-testing

**Lens**: "The Auditor"  
**Philosophy**: "Trust, but verify."

A fluid, ergonomic test harness for RustAPI applications. Don't just test your logic; test your endpoints.

## The `TestClient`

Integration testing is often painful. We make it easy. `TestClient` spawns your `RustApi` application without binding to a real TCP port, communicating directly with the service layer.

```rust
let client = TestClient::new(app);
```

## Fluent Assertions

The client provides a fluent API for making requests and asserting responses.

```rust
client.post("/login")
    .json(&credentials)
    .send()
    .await
    .assert_status(200)
    .assert_header("Set-Cookie", "session=...");
```

## Mocking Services

Because `rustapi-rs` relies heavily on Dependency Injection via `State<T>`, you can easily inject mock implementations of your database or downstream services when creating the `RustApi` instance for your test.

## Full Example

```rust
#[cfg(test)]
mod tests {
    use rustapi_testing::TestClient;
    use rustapi_rs::prelude::*;

    #[tokio::test]
    async fn test_create_user() {
        // 1. Setup app
        let app = RustApi::new().mount_route(create_user_route());
        let client = TestClient::new(app);

        // 2. Execute
        let response = client.post("/users")
            .json(&json!({ "name": "Alice" }))
            .send()
            .await;

        // 3. Assert
        response
            .assert_status(StatusCode::OK)
            .assert_json_path("$.id", 1);
    }
}
```
