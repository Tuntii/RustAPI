# rustapi-testing: The Auditor

**Lens**: "The Auditor"
**Philosophy**: "Trust, but verify."

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
