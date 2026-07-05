# Testing Strategy

Reliable software requires a robust testing strategy. RustAPI is designed to be testable at every level, from individual functions to full end-to-end scenarios.

## The Testing Pyramid

We recommend a balanced approach:
1.  **Unit Tests (70%)**: Fast, isolated tests for individual logic pieces.
2.  **Integration Tests (20%)**: Testing handlers and extractors wired together.
3.  **End-to-End (E2E) Tests (10%)**: Testing the running server from the outside.

## 1. Unit Testing Handlers

Since handlers are just regular functions, you can unit test them by invoking them directly. However, dealing with Extractors directly in tests can sometimes be verbose.

Often, it is better to extract your "Business Logic" into a separate function or trait, test that thoroughly, and keep the Handler layer thin.

```rust
// Domain Logic (Easy to test)
fn calculate_total(items: &[Item]) -> u32 {
    items.iter().map(|i| i.price).sum()
}

// Handler (Just plumbing)
async fn checkout(Json(cart): Json<Cart>) -> Json<Receipt> {
    let total = calculate_total(&cart.items);
    Json(Receipt { total })
}
```

## 2. Integration Testing with `Tower`

RustAPI routers implement `tower::Service`. This means you can send requests to your router directly in memory without spawning a TCP server or using `localhost`. This is **extremely fast**.

We rely on `tower::util::ServiceExt` to call the router.

### Setup

Add `tower` and `http-body-util` for testing utilities:

```toml
[dev-dependencies]
tower = { version = "0.4", features = ["util"] }
http-body-util = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Example Test

```rust
#[tokio::test]
async fn test_create_user() {
    // 1. Build the app (same as in main.rs)
    let app = app(); 

    // 2. Construct a Request
    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/users")
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username": "alice"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // 3. Assert Status
    assert_eq!(response.status(), StatusCode::CREATED);

    // 4. Assert Body
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: User = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body.username, "alice");
}
```

## 3. Mocking Dependencies with `State`

To test handlers that rely on databases or external APIs, you should mock those dependencies.

Use Traits to define the capabilities, and use generics or dynamic dispatch in your State.

```rust
// 1. Define the interface
#[async_trait]
trait UserRepository: Send + Sync {
    async fn get_user(&self, id: u32) -> Option<User>;
}

// 2. Real Implementation
struct PostgresRepo { pool: PgPool }

// 3. Mock Implementation
struct MockRepo;
#[async_trait]
impl UserRepository for MockRepo {
    async fn get_user(&self, _id: u32) -> Option<User> {
        Some(User { username: "mock_user".into() })
    }
}

// 4. Use in Handler
async fn get_user(
    State(repo): State<Arc<dyn UserRepository>>, // Accepts any impl
    Path(id): Path<u32>
) -> Json<User> {
    // ...
}
```

In your tests, inject `Arc::new(MockRepo)` into the `State`.

## 4. End-to-End Testing

For E2E tests, you can spawn the actual server on a random port and use a real HTTP client (like `reqwest`) to hit it.

```rust
#[tokio::test]
async fn e2e_test() {
    // Binding to port 0 lets the OS choose a random available port
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn server in background
    tokio::spawn(async move {
        RustApi::serve(listener, app()).await.unwrap();
    });

    // Make real requests
    let client = reqwest::Client::new();
    let resp = client.get(format!("http://{}/health", addr))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
}
```

This approach is slower but validates strictly everything, including network serialization and actual TCP behavior.
