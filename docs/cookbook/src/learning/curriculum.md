# Structured Learning Path

This curriculum is designed to take you from a RustAPI beginner to an advanced user capable of building production-grade microservices.

## Phase 1: Foundations

**Goal:** Build a simple CRUD API and understand the core request/response cycle.

### Module 1: Introduction & Setup
- **Prerequisites:** Rust installed, basic Cargo knowledge.
- **Reading:** [Installation](../getting_started/installation.md), [Project Structure](../getting_started/structure.md).
- **Task:** Create a new project using `cargo rustapi new my-api`.
- **Expected Output:** A running server that responds to `GET /` with "Hello World".
- **Pitfalls:** Not enabling `tokio` features if setting up manually.

### Module 2: Routing & Handlers
- **Prerequisites:** Module 1.
- **Reading:** [Handlers & Extractors](../concepts/handlers.md).
- **Task:** Create routes for `GET /users`, `POST /users`, `GET /users/{id}`.
- **Expected Output:** Endpoints that return static JSON data.
- **Pitfalls:** Forgetting to register routes in `main.rs` if not using auto-discovery.

### Module 3: Extractors
- **Prerequisites:** Module 2.
- **Reading:** [Handlers & Extractors](../concepts/handlers.md).
- **Task:** Use `Path`, `Query`, and `Json` extractors to handle dynamic input.
- **Expected Output:** `GET /users/{id}` returns the ID. `POST /users` echoes the JSON body.
- **Pitfalls:** Consuming the body twice (e.g., using `Json` and `Body` in the same handler).

## Phase 2: Core Development

**Goal:** Add real logic, validation, and documentation.

### Module 4: State Management
- **Prerequisites:** Phase 1.
- **Reading:** [State Extractor](../concepts/handlers.md).
- **Task:** Create an `AppState` struct with a `Mutex<Vec<User>>`. Inject it into handlers.
- **Expected Output:** A stateful API where POST adds a user and GET retrieves it (in-memory).
- **Pitfalls:** Using `std::sync::Mutex` instead of `tokio::sync::Mutex` in async code (though `std` is fine for simple data).

### Module 5: Validation
- **Prerequisites:** Module 4.
- **Reading:** [Validation](../crates/rustapi_validation.md).
- **Task:** Add `#[derive(Validate)]` to your `User` struct. Use `ValidatedJson`.
- **Expected Output:** Requests with invalid email or short password return `422 Unprocessable Entity`.
- **Pitfalls:** Forgetting to add `#[validate]` attributes to struct fields.

### Module 6: OpenAPI & HATEOAS
- **Prerequisites:** Module 5.
- **Reading:** [OpenAPI](../crates/rustapi_openapi.md), [Pagination Recipe](../recipes/pagination.md).
- **Task:** Add `#[derive(Schema)]` to all DTOs. Implement pagination for `GET /users`.
- **Expected Output:** Swagger UI at `/docs` showing full schema. Paginated responses with `_links`.
- **Pitfalls:** Using types that don't implement `Schema` (like raw `serde_json::Value`) inside response structs.

## Phase 3: Advanced Features

**Goal:** Security, Real-time, and Production readiness.

### Module 7: Authentication (JWT)
- **Prerequisites:** Phase 2.
- **Reading:** [JWT Auth Recipe](../recipes/jwt_auth.md).
- **Task:** Implement a login route that returns a JWT. Protect user routes with `AuthUser` extractor.
- **Expected Output:** Protected routes return `401 Unauthorized` without a valid token.
- **Pitfalls:** Hardcoding secrets. Not checking token expiration.

### Module 8: WebSockets & Real-time
- **Prerequisites:** Phase 2.
- **Reading:** [WebSockets Recipe](../recipes/websockets.md).
- **Task:** Create a chat endpoint where users can broadcast messages.
- **Expected Output:** Multiple clients connected via WS receiving messages in real-time.
- **Pitfalls:** Blocking the WebSocket loop with long-running synchronous tasks.

### Module 9: Production Readiness
- **Prerequisites:** Phase 3.
- **Reading:** [Production Tuning](../recipes/high_performance.md), [Resilience](../recipes/resilience.md).
- **Task:** Add `RateLimitLayer`, `CompressionLayer`, and `TimeoutLayer`.
- **Expected Output:** An API that handles load gracefully and rejects abuse.
- **Pitfalls:** Setting timeouts too low for slow operations.

## Knowledge Check

1.  **Q:** Which extractor consumes the request body?
    *   **A:** `Json<T>`, `ValidatedJson<T>`, `Body`.
2.  **Q:** How do you share a database connection pool across handlers?
    *   **A:** Use the `State<T>` extractor and initialize it in `RustApi::new().with_state(state)`.
3.  **Q:** What is the purpose of `derive(Schema)`?
    *   **A:** It generates the OpenAPI schema definition for the struct, allowing it to be documented in Swagger UI.
4.  **Q:** How do you handle pagination links automatically?
    *   **A:** Return `ResourceCollection<T>` and call `.with_pagination()`.

## Next Steps

*   Explore the [Examples Repository](https://github.com/Tuntii/rustapi-rs-examples).
*   Contribute a new recipe to the Cookbook!
