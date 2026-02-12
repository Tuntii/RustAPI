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

#### 🧠 Knowledge Check
1. What command scaffolds a new RustAPI project?
2. Which feature flag is required for the async runtime?
3. Where is the main entry point of the application typically located?

### Module 2: Routing & Handlers
- **Prerequisites:** Module 1.
- **Reading:** [Handlers & Extractors](../concepts/handlers.md).
- **Task:** Create routes for `GET /users`, `POST /users`, `GET /users/{id}`.
- **Expected Output:** Endpoints that return static JSON data.
- **Pitfalls:** Forgetting to register routes in `main.rs` if not using auto-discovery.

#### 🧠 Knowledge Check
1. Which macro is used to define a GET handler?
2. How do you return a JSON response from a handler?
3. What is the return type of a typical handler function?

### Module 3: Extractors
- **Prerequisites:** Module 2.
- **Reading:** [Handlers & Extractors](../concepts/handlers.md).
- **Task:** Use `Path`, `Query`, and `Json` extractors to handle dynamic input.
- **Expected Output:** `GET /users/{id}` returns the ID. `POST /users` echoes the JSON body.
- **Pitfalls:** Consuming the body twice (e.g., using `Json` and `Body` in the same handler).

#### 🧠 Knowledge Check
1. Which extractor is used for URL parameters like `/users/:id`?
2. Which extractor parses the request body as JSON?
3. Can you use multiple extractors in a single handler?

### 🏆 Phase 1 Capstone: "The Todo List API"
**Objective:** Build a simple in-memory Todo List API.
**Requirements:**
- `GET /todos`: List all todos.
- `POST /todos`: Create a new todo.
- `GET /todos/:id`: Get a specific todo.
- `DELETE /todos/:id`: Delete a todo.
- Use `State` to store the list in a `Mutex<Vec<Todo>>`.

---

## Phase 2: Core Development

**Goal:** Add real logic, validation, and documentation.

### Module 4: State Management
- **Prerequisites:** Phase 1.
- **Reading:** [State Extractor](../concepts/handlers.md).
- **Task:** Create an `AppState` struct with a `Mutex<Vec<User>>`. Inject it into handlers.
- **Expected Output:** A stateful API where POST adds a user and GET retrieves it (in-memory).
- **Pitfalls:** Using `std::sync::Mutex` instead of `tokio::sync::Mutex` in async code (though `std` is fine for simple data).

#### 🧠 Knowledge Check
1. How do you inject global state into the application?
2. Which extractor retrieves the application state?
3. Why should you use `Arc` for shared state?

### Module 5: Validation
- **Prerequisites:** Module 4.
- **Reading:** [Validation](../crates/rustapi_validation.md).
- **Task:** Add `#[derive(Validate)]` to your `User` struct. Use `ValidatedJson`.
- **Expected Output:** Requests with invalid email or short password return `422 Unprocessable Entity`.
- **Pitfalls:** Forgetting to add `#[validate]` attributes to struct fields.

#### 🧠 Knowledge Check
1. Which trait must a struct implement to be validatable?
2. What HTTP status code is returned on validation failure?
3. How do you combine JSON extraction and validation?

### Module 6: OpenAPI & HATEOAS
- **Prerequisites:** Module 5.
- **Reading:** [OpenAPI](../crates/rustapi_openapi.md), [Pagination Recipe](../recipes/pagination.md).
- **Task:** Add `#[derive(Schema)]` to all DTOs. Implement pagination for `GET /users`.
- **Expected Output:** Swagger UI at `/docs` showing full schema. Paginated responses with `_links`.
- **Pitfalls:** Using types that don't implement `Schema` (like raw `serde_json::Value`) inside response structs.

#### 🧠 Knowledge Check
1. What does `#[derive(Schema)]` do?
2. Where is the Swagger UI served by default?
3. What is HATEOAS and why is it useful?

### 🏆 Phase 2 Capstone: "The Secure Blog Engine"
**Objective:** Enhance the Todo API into a Blog Engine.
**Requirements:**
- Add `Post` resource with title, content, and author.
- Validate that titles are not empty and content is at least 10 chars.
- Add pagination to `GET /posts`.
- Enable Swagger UI to visualize the API.

---

## Phase 3: Advanced Features

**Goal:** Security, Real-time, and Production readiness.

### Module 7: Authentication (JWT)
- **Prerequisites:** Phase 2.
- **Reading:** [JWT Auth Recipe](../recipes/jwt_auth.md).
- **Task:** Implement a login route that returns a JWT. Protect user routes with `AuthUser` extractor.
- **Expected Output:** Protected routes return `401 Unauthorized` without a valid token.
- **Pitfalls:** Hardcoding secrets. Not checking token expiration.

#### 🧠 Knowledge Check
1. What is the role of the `AuthUser` extractor?
2. How do you protect a route with JWT?
3. Where should you store the JWT secret?

### Module 8: WebSockets & Real-time
- **Prerequisites:** Phase 2.
- **Reading:** [WebSockets Recipe](../recipes/websockets.md).
- **Task:** Create a chat endpoint where users can broadcast messages.
- **Expected Output:** Multiple clients connected via WS receiving messages in real-time.
- **Pitfalls:** Blocking the WebSocket loop with long-running synchronous tasks.

#### 🧠 Knowledge Check
1. How do you upgrade an HTTP request to a WebSocket connection?
2. Can you share state between HTTP handlers and WebSocket handlers?
3. What happens if a WebSocket handler panics?

### Module 9: Production Readiness & Deployment
- **Prerequisites:** Phase 3.
- **Reading:** [Production Tuning](../recipes/high_performance.md), [Resilience](../recipes/resilience.md), [Deployment](../recipes/deployment.md).
- **Task:**
    1. Add `RateLimitLayer`, `CompressionLayer`, and `TimeoutLayer`.
    2. Use `cargo rustapi deploy docker` to generate a Dockerfile.
- **Expected Output:** A resilient API ready for deployment.
- **Pitfalls:** Setting timeouts too low for slow operations.

#### 🧠 Knowledge Check
1. Why is rate limiting important?
2. What command generates a production Dockerfile?
3. How do you enable compression for responses?

### Module 10: Background Jobs & Testing
- **Prerequisites:** Phase 3.
- **Reading:** [Background Jobs Recipe](../recipes/background_jobs.md), [Testing Strategy](../concepts/testing.md).
- **Task:**
    1. Implement a job that sends a "Welcome" email (simulated) when a user registers.
    2. Write an integration test using `TestClient` to verify the registration endpoint.
- **Expected Output:** Registration returns 200 immediately; console logs show "Sending welcome email to ..." shortly after. Tests pass.
- **Pitfalls:** Forgetting to start the job worker loop.

#### 🧠 Knowledge Check
1. Why use background jobs for email sending?
2. Which backend is suitable for local development?
3. How do you enqueue a job from a handler?

### 🏆 Phase 3 Capstone: "The Real-Time Collaboration Tool"
**Objective:** Build a real-time collaborative note-taking app.
**Requirements:**
- **Auth:** Users must log in to edit notes.
- **Real-time:** Changes to a note are broadcast to all viewers via WebSockets.
- **Jobs:** When a note is deleted, schedule a background job to archive it (simulate archive).
- **Resilience:** Rate limit API requests to prevent abuse.
- **Deployment:** specify a `Dockerfile` for the application.

---

## Next Steps

*   Explore the [Examples Repository](https://github.com/Tuntii/rustapi-rs-examples).
*   Contribute a new recipe to the Cookbook!
