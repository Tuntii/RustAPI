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

#### 🛠️ Mini Project: "The Echo Server"
Create a new endpoint `POST /echo` that accepts any text body and returns it back to the client. This verifies your setup handles basic I/O correctly.

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

#### 🛠️ Mini Project: "The Calculator"
Create an endpoint `GET /add?a=5&b=10` that returns `{"result": 15}`. This practices query parameter extraction and JSON responses.

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

#### 🛠️ Mini Project: "The User Registry"
Create a `POST /register` endpoint that accepts a JSON body `{"username": "...", "age": ...}` and returns a welcome message using the username. Use the `Json` extractor.

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

### Module 4.5: Database Integration
- **Prerequisites:** Module 4.
- **Reading:** [Database Integration](../recipes/db_integration.md).
- **Task:** Replace the in-memory `Mutex<Vec<User>>` with a PostgreSQL connection pool (`sqlx::PgPool`).
- **Expected Output:** Data persists across server restarts.
- **Pitfalls:** Blocking the async runtime with synchronous DB drivers (use `sqlx` or `tokio-postgres`).

#### 🧠 Knowledge Check
1. Why is connection pooling important?
2. How do you share a DB pool across handlers?
3. What is the benefit of compile-time query checking in SQLx?

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

### Module 5.5: Error Handling
- **Prerequisites:** Module 5.
- **Reading:** [Error Handling](../concepts/errors.md).
- **Task:** Create a custom `ApiError` enum and implement `IntoResponse`. Return robust error messages.
- **Expected Output:** `GET /users/999` returns `404 Not Found` with a structured JSON error body.
- **Pitfalls:** Exposing internal database errors (like SQL strings) to the client.

#### 🧠 Knowledge Check
1. What is the standard error type in RustAPI?
2. How do you mask internal errors in production?
3. What is the purpose of the `error_id` field?

### Module 6: OpenAPI & HATEOAS
- **Prerequisites:** Module 5.
- **Reading:** [OpenAPI](../crates/rustapi_openapi.md), [OpenAPI Refs](../recipes/openapi_refs.md), [Pagination Recipe](../recipes/pagination.md).
- **Task:** Add `#[derive(Schema)]` to all DTOs. Use `#[derive(Schema)]` on a shared struct and reference it in multiple places.
- **Expected Output:** Swagger UI at `/docs` showing full schema with shared components.
- **Pitfalls:** Recursive schemas without `Box` or `Option`.

#### 🧠 Knowledge Check
1. What does `#[derive(Schema)]` do?
2. How does RustAPI handle shared schema components?
3. What is HATEOAS and why is it useful?

### Module 6.5: File Uploads & Multipart
- **Prerequisites:** Module 6.
- **Reading:** [File Uploads](../recipes/file_uploads.md).
- **Task:** Create an endpoint `POST /upload` that accepts a file and saves it to disk.
- **Expected Output:** `curl -F file=@image.png` uploads the file.
- **Pitfalls:** Loading large files entirely into memory (use streaming).

#### 🧠 Knowledge Check
1. Which extractor is used for file uploads?
2. Why should you use `field.chunk()` instead of `field.bytes()`?
3. How do you increase the request body size limit?

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

### Module 7: Authentication (JWT & OAuth2)
- **Prerequisites:** Phase 2.
- **Reading:** [JWT Auth Recipe](../recipes/jwt_auth.md), [OAuth2 Client](../recipes/oauth2_client.md).
- **Task:**
    1. Implement a login route that returns a JWT.
    2. Protect user routes with `AuthUser` extractor.
    3. (Optional) Implement "Login with Google" using `OAuth2Client`.
- **Expected Output:** Protected routes return `401 Unauthorized` without a valid token.
- **Pitfalls:** Hardcoding secrets. Not checking token expiration.

#### 🧠 Knowledge Check
1. What is the role of the `AuthUser` extractor?
2. How does OAuth2 PKCE improve security?
3. Where should you store the JWT secret?

### Module 8: Advanced Middleware
- **Prerequisites:** Module 7.
- **Reading:** [Advanced Middleware](../recipes/advanced_middleware.md).
- **Task:**
    1. Apply `RateLimitLayer` to your login endpoint (10 requests/minute).
    2. Add `DedupLayer` to a payment endpoint.
    3. Cache the response of a public "stats" endpoint.
- **Expected Output:** Sending 11 login attempts results in `429 Too Many Requests`.
- **Pitfalls:** Caching responses that contain user-specific data.

#### 🧠 Knowledge Check
1. What header indicates when the rate limit resets?
2. Why is request deduplication important for payments?
3. Which requests are typically safe to cache?

### Module 9: WebSockets & Real-time
- **Prerequisites:** Phase 2.
- **Reading:** [WebSockets Recipe](../recipes/websockets.md).
- **Task:** Create a chat endpoint where users can broadcast messages.
- **Expected Output:** Multiple clients connected via WS receiving messages in real-time.
- **Pitfalls:** Blocking the WebSocket loop with long-running synchronous tasks.

#### 🧠 Knowledge Check
1. How do you upgrade an HTTP request to a WebSocket connection?
2. Can you share state between HTTP handlers and WebSocket handlers?
3. What happens if a WebSocket handler panics?

### Module 10: Production Readiness & Deployment
- **Prerequisites:** Phase 3.
- **Reading:** [Production Tuning](../recipes/high_performance.md), [Resilience](../recipes/resilience.md), [Deployment](../recipes/deployment.md).
- **Task:**
    1. Add `CompressionLayer`, and `TimeoutLayer`.
    2. Use `cargo rustapi deploy docker` to generate a Dockerfile.
- **Expected Output:** A resilient API ready for deployment.
- **Pitfalls:** Setting timeouts too low for slow operations.

#### 🧠 Knowledge Check
1. Why is timeout middleware important?
2. What command generates a production Dockerfile?
3. How do you enable compression for responses?

### Module 11: Background Jobs & Testing
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
- **Auth:** Users must log in (JWT or OAuth2) to edit notes.
- **Real-time:** Changes to a note are broadcast to all viewers via WebSockets.
- **Jobs:** When a note is deleted, schedule a background job to archive it (simulate archive).
- **Resilience:** Rate limit API requests to prevent abuse.
- **Deployment:** specify a `Dockerfile` for the application.

---

## Phase 4: Enterprise Scale

**Goal:** Build observable, resilient, and high-performance distributed systems.

### Module 12: Observability & Auditing
- **Prerequisites:** Phase 3.
- **Reading:** [Observability (Extras)](../crates/rustapi_extras.md#observability), [Audit Logging](../recipes/audit_logging.md).
- **Task:**
    1. Enable `structured-logging` and `otel`.
    2. Configure tracing to export spans.
    3. Implement `AuditStore` and log a "User Login" event with IP address.
- **Expected Output:** Logs are JSON formatted. Audit log contains a new entry for every login.
- **Pitfalls:** High cardinality in metric labels.

#### 🧠 Knowledge Check
1. What is the difference between logging and auditing?
2. Which fields are required in an `AuditEvent`?
3. How does structured logging aid debugging?

### Module 13: Resilience & Security
- **Prerequisites:** Phase 3.
- **Reading:** [Resilience Patterns](../recipes/resilience.md), [Time-Travel Debugging](../recipes/replay.md).
- **Task:**
    1. Wrap an external API call with a `CircuitBreaker`.
    2. Implement `RetryLayer` for transient failures.
    3. (Optional) Use `ReplayLayer` to record and replay a tricky bug scenario.
- **Expected Output:** System degrades gracefully when external service is down. Replay file captures the exact request sequence.
- **Pitfalls:** Infinite retry loops or retrying non-idempotent operations.

#### 🧠 Knowledge Check
1. What state does a Circuit Breaker have when it stops traffic?
2. Why is jitter important in retry strategies?
3. How does Time-Travel Debugging help with "Heisenbugs"?

### Module 14: High Performance
- **Prerequisites:** Phase 3.
- **Reading:** [HTTP/3 (QUIC)](../recipes/http3_quic.md), [Performance Tuning](../recipes/high_performance.md), [Compression](../recipes/compression.md).
- **Task:**
    1. Enable `http3` feature and generate self-signed certs.
    2. Serve traffic over QUIC.
    3. Add `CompressionLayer` to compress large responses.
- **Expected Output:** Browser/Client connects via HTTP/3. Responses have `content-encoding: gzip`.
- **Pitfalls:** Compressing small responses (waste of CPU) or already compressed data (images).

#### 🧠 Knowledge Check
1. What transport protocol does HTTP/3 use?
2. How does `simd-json` improve performance?
3. Why shouldn't you compress JPEG images?

### 🏆 Phase 4 Capstone: "The High-Scale Event Platform"
**Objective:** Architect a system capable of handling thousands of events per second.
**Requirements:**
- **Ingestion:** HTTP/3 endpoint receiving JSON events.
- **Processing:** Push events to a `rustapi-jobs` queue (Redis backend).
- **Storage:** Workers process events and store aggregates in a database.
- **Observability:** Full tracing from ingestion to storage.
- **Audit:** Log all configuration changes to the system.
- **Resilience:** Circuit breakers on database writes.
- **Testing:** Load test the ingestion endpoint (e.g., with k6 or similar) and observe metrics.

---

## Phase 5: Specialized Skills

**Goal:** Master integration with AI, gRPC, and server-side rendering.

### Module 15: Server-Side Rendering (SSR)
- **Prerequisites:** Phase 2.
- **Reading:** [SSR Recipe](../recipes/server_side_rendering.md).
- **Task:** Create a dashboard showing system status using `rustapi-view`.
- **Expected Output:** HTML page rendered with Tera templates, displaying dynamic data.
- **Pitfalls:** Forgetting to create the `templates/` directory.

#### 🧠 Knowledge Check
1. Which template engine does RustAPI use?
2. How do you pass data to a template?
3. How does template reloading work in debug mode?

### Module 16: gRPC Microservices
- **Prerequisites:** Phase 3.
- **Reading:** [gRPC Recipe](../recipes/grpc_integration.md).
- **Task:** Run a gRPC service alongside your HTTP API that handles internal user lookups.
- **Expected Output:** Both servers running; HTTP endpoint calls gRPC method (simulated).
- **Pitfalls:** Port conflicts if not configured correctly.

#### 🧠 Knowledge Check
1. Which crate provides gRPC helpers for RustAPI?
2. Can HTTP and gRPC share the same Tokio runtime?
3. Why might you want to run both in the same process?

### Module 17: AI Integration (TOON)
- **Prerequisites:** Phase 2.
- **Reading:** [AI Integration Recipe](../recipes/ai_integration.md).
- **Task:** Create an endpoint that returns standard JSON for browsers but TOON for `Accept: application/toon`.
- **Expected Output:** `curl` requests with different headers return different formats.
- **Pitfalls:** Not checking the `Accept` header in client code.

#### 🧠 Knowledge Check
1. What is TOON and why is it useful for LLMs?
2. How does `LlmResponse` decide which format to return?
3. How much token usage can TOON save on average?

### 🏆 Phase 5 Capstone: "The Intelligent Dashboard"
**Objective:** Combine SSR, gRPC, and AI features.
**Requirements:**
- **Backend:** Retrieve stats via gRPC from a "worker" service.
- **Frontend:** Render a dashboard using SSR.
- **AI Agent:** Expose a TOON endpoint for an AI agent to query the system status.

---

## Next Steps

*   Explore the [Examples Repository](https://github.com/Tuntii/rustapi-rs-examples).
*   Contribute a new recipe to the Cookbook!
