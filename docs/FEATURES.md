# RustAPI Features

> Complete reference for all RustAPI features and capabilities.

---

## Table of Contents

1. [Extractors](#extractors)
2. [Response Types](#response-types)
3. [Validation](#validation)
4. [OpenAPI & Swagger](#openapi--swagger)
5. [Middleware](#middleware)
6. [TOON Format](#toon-format)
7. [WebSocket](#websocket)
8. [Template Engine](#template-engine)
9. [Testing](#testing)
10. [Error Handling](#error-handling)
11. [Configuration](#configuration)

---

## Extractors

Extractors parse incoming requests into typed Rust values.

### `Json<T>`

Extract JSON body.

```rust
#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

#[rustapi_rs::post("/users")]
async fn create_user(Json(body): Json<CreateUser>) -> Json<User> {
    // body is parsed and typed
}
```

**Errors:**
- 400 Bad Request — Invalid JSON syntax
- 415 Unsupported Media Type — Missing `Content-Type: application/json`
- 422 Unprocessable Entity — JSON doesn't match schema

### `Path<T>`

Extract URL path parameters.

```rust
// Single parameter
#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<u64>) -> Json<User> { ... }

// Multiple parameters
#[derive(Deserialize)]
struct PathParams {
    org: String,
    repo: String,
}

#[rustapi_rs::get("/orgs/{org}/repos/{repo}")]
async fn get_repo(Path(params): Path<PathParams>) -> Json<Repo> { ... }

// Tuple extraction
#[rustapi_rs::get("/users/{user_id}/posts/{post_id}")]
async fn get_post(Path((user_id, post_id)): Path<(u64, u64)>) -> Json<Post> { ... }
```

### `Query<T>`

Extract query string parameters.

```rust
#[derive(Deserialize)]
struct Filters {
    page: Option<u32>,
    limit: Option<u32>,
    search: Option<String>,
}

#[rustapi_rs::get("/users")]
async fn list_users(Query(filters): Query<Filters>) -> Json<Vec<User>> {
    let page = filters.page.unwrap_or(1);
    let limit = filters.limit.unwrap_or(10);
    // ...
}
```

URL: `/users?page=2&limit=20&search=alice`

### `State<T>`

Extract application state.

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

type AppState = Arc<RwLock<Vec<User>>>;

#[tokio::main]
async fn main() {
    let state: AppState = Arc::new(RwLock::new(vec![]));
    
    RustApi::new()
        .state(state)
        .route("/users", get(list_users))
        .run("0.0.0.0:8080")
        .await
}

async fn list_users(State(db): State<AppState>) -> Json<Vec<User>> {
    let users = db.read().await;
    Json(users.clone())
}
```

### `Body`

Extract raw request body as bytes.

```rust
use bytes::Bytes;

#[rustapi_rs::post("/upload")]
async fn upload(body: Body) -> &'static str {
    let bytes: Bytes = body.into_bytes().await;
    // Process raw bytes
    "Uploaded"
}
```

### `Headers`

Extract request headers.

```rust
use rustapi_rs::extract::Headers;

#[rustapi_rs::get("/info")]
async fn info(headers: Headers) -> Json<Info> {
    let user_agent = headers.get("user-agent").unwrap_or("unknown");
    let auth = headers.get("authorization");
    // ...
}
```

### `Cookies`

Extract cookies (requires `cookies` feature).

```rust
use rustapi_rs::extract::Cookies;

#[rustapi_rs::get("/session")]
async fn session(cookies: Cookies) -> Json<Session> {
    let session_id = cookies.get("session_id");
    // ...
}
```

### `ClientIp`

Extract client IP address.

```rust
use rustapi_rs::extract::ClientIp;

#[rustapi_rs::get("/whoami")]
async fn whoami(ClientIp(ip): ClientIp) -> String {
    format!("Your IP: {}", ip)
}
```

Checks `X-Forwarded-For`, `X-Real-IP`, then socket address.

### `AuthUser<T>` (JWT)

Extract authenticated user claims (requires `jwt` feature).

```rust
use rustapi_rs::extract::AuthUser;

#[derive(Deserialize)]
struct Claims {
    sub: String,
    role: String,
    exp: u64,
}

#[rustapi_rs::get("/profile")]
async fn profile(user: AuthUser<Claims>) -> Json<Profile> {
    // user.claims contains decoded JWT claims
    Json(Profile {
        user_id: user.claims.sub,
        role: user.claims.role,
    })
}
```

### `ValidatedJson<T>`

Extract and validate JSON body.

```rust
use rustapi_rs::extract::ValidatedJson;

#[derive(Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 3))]
    name: String,
    #[validate(email)]
    email: String,
}

#[rustapi_rs::post("/users")]
async fn create_user(ValidatedJson(body): ValidatedJson<CreateUser>) -> Json<User> {
    // body is guaranteed to pass validation
}
```

---

## Response Types

### `Json<T>`

Standard JSON response (200 OK).

```rust
#[derive(Serialize)]
struct User { id: u64, name: String }

async fn get_user() -> Json<User> {
    Json(User { id: 1, name: "Alice".into() })
}
```

### `Created<T>`

201 Created with JSON body.

```rust
use rustapi_rs::response::Created;

async fn create_user(Json(body): Json<CreateUser>) -> Created<User> {
    let user = User { id: 1, name: body.name };
    Created(user)
}
```

### `NoContent`

204 No Content.

```rust
use rustapi_rs::response::NoContent;

async fn delete_user(Path(id): Path<u64>) -> NoContent {
    // delete user...
    NoContent
}
```

### `Html<T>`

HTML response.

```rust
use rustapi_rs::response::Html;

async fn page() -> Html<String> {
    Html("<h1>Hello, World!</h1>".into())
}
```

### `Redirect`

HTTP redirect.

```rust
use rustapi_rs::response::Redirect;

async fn old_route() -> Redirect {
    Redirect::permanent("/new-route")
}

async fn temp_redirect() -> Redirect {
    Redirect::temporary("/maintenance")
}
```

### Plain Text

```rust
async fn health() -> &'static str {
    "OK"
}

async fn dynamic_text() -> String {
    format!("Server time: {}", chrono::Utc::now())
}
```

### Tuples

```rust
use rustapi_rs::http::StatusCode;

async fn custom_response() -> (StatusCode, Json<Message>) {
    (StatusCode::CREATED, Json(Message { text: "Created!".into() }))
}

async fn with_headers() -> (StatusCode, [(&'static str, &'static str); 1], String) {
    (StatusCode::OK, [("X-Custom", "value")], "Hello".into())
}
```

### Result

```rust
async fn fallible() -> Result<Json<User>> {
    let user = find_user(1).ok_or(ApiError::not_found("User not found"))?;
    Ok(Json(user))
}
```

---

## Validation

Built-in validation using the `Validate` derive macro.

### Basic Validation

```rust
#[derive(Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 1, max = 100))]
    name: String,
    
    #[validate(email)]
    email: String,
    
    #[validate(range(min = 0, max = 150))]
    age: u8,
}
```

### Available Validators

| Validator | Example | Description |
|-----------|---------|-------------|
| `email` | `#[validate(email)]` | Valid email format |
| `url` | `#[validate(url)]` | Valid URL |
| `length` | `#[validate(length(min = 1, max = 100))]` | String/array length |
| `range` | `#[validate(range(min = 0, max = 999))]` | Numeric range |
| `regex` | `#[validate(regex(path = "PHONE_RE"))]` | Regex pattern |
| `contains` | `#[validate(contains = "@")]` | Contains substring |
| `must_match` | `#[validate(must_match = "password")]` | Fields must match |
| `custom` | `#[validate(custom(function = "fn"))]` | Custom validator |

### Custom Validators

```rust
fn validate_username(username: &str) -> Result<(), validator::ValidationError> {
    if username.starts_with("admin") {
        return Err(validator::ValidationError::new("reserved_username"));
    }
    Ok(())
}

#[derive(Deserialize, Validate)]
struct CreateUser {
    #[validate(custom(function = "validate_username"))]
    username: String,
}
```

### Nested Validation

```rust
#[derive(Deserialize, Validate)]
struct Address {
    #[validate(length(min = 1))]
    street: String,
    #[validate(length(min = 1))]
    city: String,
}

#[derive(Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 1))]
    name: String,
    
    #[validate]  // Validates nested struct
    address: Address,
}
```

### Error Response

Validation failures return 422 Unprocessable Entity:

```json
{
    "status": 422,
    "error_type": "validation_error",
    "message": "Request validation failed",
    "error_id": "err_abc123",
    "fields": [
        {
            "field": "email",
            "message": "Invalid email format"
        },
        {
            "field": "age",
            "message": "Must be between 0 and 150"
        }
    ]
}
```

---

## OpenAPI & Swagger

Automatic API documentation generation.

### Schema Derivation

```rust
#[derive(Serialize, Schema)]
struct User {
    /// The user's unique identifier
    id: u64,
    
    /// The user's display name
    name: String,
    
    #[schema(format = "email")]
    email: String,
    
    #[schema(example = "2024-01-01T00:00:00Z")]
    created_at: String,
}
```

### Route Documentation

```rust
#[rustapi_rs::get("/users/{id}")]
#[doc = "Get a user by ID"]
async fn get_user(
    /// The user ID to fetch
    Path(id): Path<u64>,
) -> Json<User> {
    // ...
}
```

### Accessing Documentation

- **Swagger UI:** `http://localhost:8080/docs`
- **OpenAPI JSON:** `http://localhost:8080/openapi.json`

### Custom OpenAPI Info

```rust
RustApi::new()
    .openapi_info(|info| {
        info.title("My API")
            .version("1.0.0")
            .description("My awesome API")
    })
    .run("0.0.0.0:8080")
    .await
```

---

## Middleware

### CorsLayer

Cross-Origin Resource Sharing.

```rust
use rustapi_rs::middleware::CorsLayer;

// Allow all origins
RustApi::new()
    .layer(CorsLayer::permissive())
    .run("0.0.0.0:8080")
    .await

// Custom configuration
RustApi::new()
    .layer(
        CorsLayer::new()
            .allow_origin("https://example.com")
            .allow_origin("https://app.example.com")
            .allow_methods(["GET", "POST", "PUT", "DELETE"])
            .allow_headers(["Content-Type", "Authorization"])
            .allow_credentials(true)
            .max_age(Duration::from_secs(3600))
    )
    .run("0.0.0.0:8080")
    .await
```

### JwtLayer

JWT authentication.

```rust
use rustapi_rs::middleware::JwtLayer;

RustApi::new()
    .layer(
        JwtLayer::new("your-secret-key")
            .skip_paths(["/login", "/register", "/health"])
            .algorithm(Algorithm::HS256)  // Default
    )
    .run("0.0.0.0:8080")
    .await
```

### RateLimitLayer

IP-based rate limiting.

```rust
use rustapi_rs::middleware::RateLimitLayer;
use std::time::Duration;

RustApi::new()
    .layer(RateLimitLayer::new(100, Duration::from_secs(60)))  // 100 req/min
    .run("0.0.0.0:8080")
    .await
```

### BodyLimitLayer

Limit request body size.

```rust
use rustapi_rs::middleware::BodyLimitLayer;

RustApi::new()
    .layer(BodyLimitLayer::new(1024 * 1024))  // 1 MB
    .run("0.0.0.0:8080")
    .await
```

### RequestIdLayer

Add unique request IDs.

```rust
use rustapi_rs::middleware::RequestIdLayer;

RustApi::new()
    .layer(RequestIdLayer::new())
    .run("0.0.0.0:8080")
    .await

// Adds X-Request-ID header to responses
```

### TracingLayer

Request/response logging.

```rust
use rustapi_rs::middleware::TracingLayer;

RustApi::new()
    .layer(TracingLayer::new())
    .run("0.0.0.0:8080")
    .await

// Logs: method, path, status, duration
```

### MetricsLayer

Prometheus metrics.

```rust
use rustapi_rs::middleware::MetricsLayer;

RustApi::new()
    .layer(MetricsLayer::new())
    .route("/metrics", get(metrics_handler))
    .run("0.0.0.0:8080")
    .await

// Metrics:
// - http_requests_total{method, path, status}
// - http_request_duration_seconds{method, path}
```

### Middleware Order

Middleware executes in order added (first added = outermost):

```rust
RustApi::new()
    .layer(RequestIdLayer::new())   // 1st - Adds request ID
    .layer(TracingLayer::new())     // 2nd - Logs request
    .layer(CorsLayer::permissive()) // 3rd - Handles CORS
    .layer(RateLimitLayer::new(...))// 4th - Rate limiting
    .layer(JwtLayer::new(...))      // 5th - Authentication
```

---

## TOON Format

Token-Oriented Object Notation for LLM optimization.

### Basic Usage

```rust
use rustapi_rs::toon::Toon;

#[rustapi_rs::get("/ai/users")]
async fn ai_users() -> Toon<UsersResponse> {
    Toon(get_users())
}
```

### Content Negotiation

```rust
use rustapi_rs::toon::{LlmResponse, AcceptHeader};

#[rustapi_rs::get("/users")]
async fn users(accept: AcceptHeader) -> LlmResponse<UsersResponse> {
    // Automatically chooses JSON or TOON based on Accept header
    LlmResponse::new(get_users(), accept.preferred)
}
```

### Token Counting Headers

`LlmResponse` adds these headers:

| Header | Description |
|--------|-------------|
| `X-Token-Count-JSON` | Approximate tokens if JSON |
| `X-Token-Count-TOON` | Approximate tokens if TOON |
| `X-Token-Savings` | Percentage saved (e.g., "57.8%") |
| `X-Format-Used` | "json" or "toon" |

### Accept Header Values

| Accept Header | Result |
|---------------|--------|
| `application/json` | JSON response |
| `application/toon` | TOON response |
| `*/*` | Default (JSON) |
| `application/toon, application/json` | TOON (preferred) |

### Format Comparison

**JSON:**
```json
{"users":[{"id":1,"name":"Alice","email":"alice@example.com"},{"id":2,"name":"Bob","email":"bob@example.com"}],"total":2,"page":1}
```

**TOON:**
```
users[(id:1,name:Alice,email:alice@example.com)(id:2,name:Bob,email:bob@example.com)]total:2,page:1
```

**Savings:** ~50-58% fewer tokens

---

## WebSocket

Real-time bidirectional communication support (requires `ws` feature).

### Basic WebSocket Handler

```rust
use rustapi_rs::ws::{WebSocket, WebSocketUpgrade, WebSocketStream, Message};

#[rustapi_rs::get("/ws")]
async fn websocket(ws: WebSocket) -> WebSocketUpgrade {
    ws.on_upgrade(handle_connection)
}

async fn handle_connection(mut stream: WebSocketStream) {
    while let Some(msg) = stream.recv().await {
        match msg {
            Message::Text(text) => {
                // Echo the message back
                stream.send(Message::Text(format!("Echo: {}", text))).await.ok();
            }
            Message::Binary(data) => {
                // Handle binary data
                stream.send(Message::Binary(data)).await.ok();
            }
            Message::Ping(data) => {
                stream.send(Message::Pong(data)).await.ok();
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
```

### Message Types

| Type | Description |
|------|-------------|
| `Message::Text(String)` | UTF-8 text message |
| `Message::Binary(Vec<u8>)` | Binary data |
| `Message::Ping(Vec<u8>)` | Ping frame (keepalive) |
| `Message::Pong(Vec<u8>)` | Pong response |
| `Message::Close(Option<CloseFrame>)` | Connection close |

### Broadcast Channel

For pub/sub patterns (chat rooms, live updates):

```rust
use rustapi_rs::ws::{Broadcast, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let broadcast = Arc::new(Broadcast::new());
    
    RustApi::new()
        .state(broadcast)
        .route("/ws", get(websocket))
        .route("/broadcast", post(send_broadcast))
        .run("0.0.0.0:8080")
        .await
}

#[rustapi_rs::get("/ws")]
async fn websocket(
    ws: WebSocket,
    State(broadcast): State<Arc<Broadcast>>,
) -> WebSocketUpgrade {
    let mut rx = broadcast.subscribe();
    ws.on_upgrade(move |mut stream| async move {
        loop {
            tokio::select! {
                // Receive from client
                msg = stream.recv() => {
                    match msg {
                        Some(Message::Close(_)) | None => break,
                        _ => {}
                    }
                }
                // Receive broadcasts
                Ok(msg) = rx.recv() => {
                    if stream.send(msg).await.is_err() {
                        break;
                    }
                }
            }
        }
    })
}

#[rustapi_rs::post("/broadcast")]
async fn send_broadcast(
    State(broadcast): State<Arc<Broadcast>>,
    body: String,
) -> &'static str {
    broadcast.send(Message::Text(body));
    "Sent"
}
```

### WebSocket with State

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct ConnectionCounter(AtomicUsize);

#[rustapi_rs::get("/ws")]
async fn websocket(
    ws: WebSocket,
    State(counter): State<Arc<ConnectionCounter>>,
) -> WebSocketUpgrade {
    ws.on_upgrade(move |stream| async move {
        counter.0.fetch_add(1, Ordering::SeqCst);
        handle_connection(stream).await;
        counter.0.fetch_sub(1, Ordering::SeqCst);
    })
}
```

---

## Template Engine

Server-side HTML rendering with Tera templates (requires `view` feature).

### Setup

```rust
use rustapi_rs::view::{Templates, TemplatesConfig};

#[tokio::main]
async fn main() {
    let templates = Templates::new(TemplatesConfig {
        directory: "templates".into(),
        extension: "html".into(),
    }).expect("Failed to load templates");
    
    RustApi::new()
        .state(templates)
        .route("/", get(home))
        .run("0.0.0.0:8080")
        .await
}
```

### Basic Template Rendering

```rust
use rustapi_rs::view::{Templates, View};

#[rustapi_rs::get("/")]
async fn home(templates: Templates) -> View<()> {
    View::new(&templates, "index.html", ())
}

#[derive(Serialize)]
struct UserData {
    name: String,
    email: String,
}

#[rustapi_rs::get("/user/{id}")]
async fn user_page(
    templates: Templates,
    Path(id): Path<u64>,
) -> View<UserData> {
    let user = UserData {
        name: "Alice".into(),
        email: "alice@example.com".into(),
    };
    View::new(&templates, "user.html", user)
}
```

### Template with Extra Context

```rust
use rustapi_rs::view::{Templates, View, ContextBuilder};

#[rustapi_rs::get("/dashboard")]
async fn dashboard(templates: Templates) -> View<DashboardData> {
    let data = get_dashboard_data();
    
    View::with_context(&templates, "dashboard.html", data, |ctx| {
        ctx.insert("title", &"Dashboard");
        ctx.insert("year", &2024);
        ctx.insert("nav_items", &vec!["Home", "Users", "Settings"]);
    })
}
```

### Tera Template Syntax

**templates/base.html:**
```html
<!DOCTYPE html>
<html>
<head>
    <title>{% block title %}My App{% endblock %}</title>
</head>
<body>
    <nav>{% block nav %}{% endblock %}</nav>
    <main>{% block content %}{% endblock %}</main>
</body>
</html>
```

**templates/user.html:**
```html
{% extends "base.html" %}

{% block title %}{{ name }} - My App{% endblock %}

{% block content %}
<div class="user-profile">
    <h1>{{ name }}</h1>
    <p>Email: {{ email }}</p>
    
    {% if posts %}
    <h2>Posts</h2>
    <ul>
        {% for post in posts %}
        <li>{{ post.title }}</li>
        {% endfor %}
    </ul>
    {% endif %}
</div>
{% endblock %}
```

### Template Features

| Feature | Syntax | Description |
|---------|--------|-------------|
| Variables | `{{ name }}` | Output variable |
| Filters | `{{ name \| upper }}` | Transform values |
| Conditionals | `{% if x %}...{% endif %}` | Conditional rendering |
| Loops | `{% for x in items %}` | Iteration |
| Inheritance | `{% extends "base.html" %}` | Template inheritance |
| Blocks | `{% block name %}` | Overridable sections |
| Includes | `{% include "partial.html" %}` | Include templates |
| Macros | `{% macro name() %}` | Reusable snippets |

### Built-in Filters

| Filter | Example | Description |
|--------|---------|-------------|
| `upper` | `{{ name \| upper }}` | UPPERCASE |
| `lower` | `{{ name \| lower }}` | lowercase |
| `capitalize` | `{{ name \| capitalize }}` | Capitalize |
| `trim` | `{{ text \| trim }}` | Remove whitespace |
| `length` | `{{ items \| length }}` | Array/string length |
| `default` | `{{ x \| default(value="N/A") }}` | Default value |
| `date` | `{{ dt \| date(format="%Y-%m-%d") }}` | Date formatting |
| `json_encode` | `{{ obj \| json_encode }}` | JSON string |

### Error Handling

```rust
#[rustapi_rs::get("/user/{id}")]
async fn user_page(
    templates: Templates,
    Path(id): Path<u64>,
) -> Result<View<UserData>> {
    let user = find_user(id)
        .ok_or_else(|| ApiError::not_found("User not found"))?;
    
    Ok(View::new(&templates, "user.html", user))
}
```

---

## Testing

### Using `rustapi-testing` (Recommended)

```rust
use rustapi_testing::{TestServer, Matcher};

#[tokio::test]
async fn test_api() {
    let server = TestServer::new(app()).await;
    
    let response = server
        .get("/users/1")
        .send()
        .await;
    
    response
        .assert_status(200)
        .assert_json(Matcher::object()
            .field("id", 1)
            .field("name", Matcher::string()));
}
```

### Expectation Builder

```rust
use rustapi_testing::Expectation;

Expectation::new()
    .method("POST")
    .path("/users")
    .body_json(json!({ "name": "Alice" }))
    .expect_status(201)
    .expect_header("Location", "/users/1");
```

### TestClient (Legacy)

```rust
use rustapi_rs::test::TestClient;

#[tokio::test]
async fn test_get_user() {
    let app = RustApi::new()
        .route("/users/{id}", get(get_user));
    
    let client = TestClient::new(app);
    
    // GET request
    let res = client.get("/users/1").send().await;
    assert_eq!(res.status(), 200);
    
    // Parse JSON
    let user: User = res.json().await;
    assert_eq!(user.id, 1);
}
```

### POST with JSON

```rust
#[tokio::test]
async fn test_create_user() {
    let app = RustApi::new()
        .route("/users", post(create_user));
    
    let client = TestClient::new(app);
    
    let res = client
        .post("/users")
        .json(&CreateUser { name: "Alice".into() })
        .send()
        .await;
    
    assert_eq!(res.status(), 201);
}
```

### With Headers

```rust
#[tokio::test]
async fn test_with_auth() {
    let client = TestClient::new(app);
    
    let res = client
        .get("/protected")
        .header("Authorization", "Bearer token123")
        .send()
        .await;
    
    assert_eq!(res.status(), 200);
}
```

### With State

```rust
#[tokio::test]
async fn test_with_mock() {
    let mock_db = MockDb::new();
    mock_db.insert(User { id: 1, name: "Test".into() });
    
    let app = RustApi::new()
        .state(mock_db)
        .route("/users/{id}", get(get_user));
    
    let client = TestClient::new(app);
    // ...
}
```

---

## Error Handling

### ApiError

```rust
pub struct ApiError {
    pub status: u16,
    pub error_type: String,
    pub message: String,
    pub error_id: String,
    pub fields: Option<Vec<FieldError>>,
}
```

### Error Constructors

```rust
ApiError::bad_request("message")          // 400
ApiError::unauthorized("message")         // 401
ApiError::forbidden("message")            // 403
ApiError::not_found("message")            // 404
ApiError::method_not_allowed("message")   // 405
ApiError::conflict("message")             // 409
ApiError::unprocessable("message")        // 422
ApiError::too_many_requests("message")    // 429
ApiError::internal("message")             // 500
```

### Custom Error

```rust
ApiError::new(StatusCode::IM_A_TEAPOT, "teapot", "I'm a teapot")
```

### Result Type

```rust
use rustapi_rs::prelude::*;

async fn handler() -> Result<Json<User>> {
    let user = db.find(1)
        .ok_or(ApiError::not_found("User not found"))?;
    Ok(Json(user))
}
```

### Error Masking (Production)

Set `RUSTAPI_ENV=production` to mask internal errors:

```rust
// Development: Full error details
// Production: "Internal server error" + error_id (details in logs)
```

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUSTAPI_ENV` | `development` | `production` masks errors |
| `RUSTAPI_LOG` | `info` | Log level |
| `RUSTAPI_BODY_LIMIT` | `1048576` | Max body size (bytes) |
| `RUSTAPI_REQUEST_TIMEOUT` | `30` | Request timeout (seconds) |

### Programmatic Configuration

```rust
RustApi::new()
    .body_limit(5 * 1024 * 1024)  // 5 MB
    .request_timeout(Duration::from_secs(60))
    .run("0.0.0.0:8080")
    .await
```

### Feature Flags

```toml
[dependencies]
rustapi-rs = { version = "0.1.4", features = ["full"] }
```

| Feature | Description |
|---------|-------------|
| `swagger-ui` | Swagger UI (default) |
| `jwt` | JWT authentication |
| `cors` | CORS middleware |
| `rate-limit` | Rate limiting |
| `toon` | TOON format |
| `cookies` | Cookie extraction |
| `ws` | WebSocket support |
| `view` | Template engine (Tera) |
| `simd-json` | 2-4x faster JSON parsing |
| `audit` | GDPR/SOC2 audit logging |
| `full` | All features |

---

## Background Jobs

Process tasks asynchronously with `rustapi-jobs`.

### Basic Usage

```rust
use rustapi_jobs::{Job, JobQueue, MemoryBackend};

// Define a job
#[derive(Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
}

// Create queue with in-memory backend (dev)
let queue = JobQueue::new(MemoryBackend::new());

// Enqueue a job
queue.push(Job::new("send_email", SendEmailJob {
    to: "user@example.com".into(),
    subject: "Welcome!".into(),
})).await?;

// Process jobs
queue.process(|job| async move {
    // Handle job based on type
    Ok(())
}).await;
```

### Redis Backend (Production)

```rust
use rustapi_jobs::{JobQueue, RedisBackend};

let backend = RedisBackend::new("redis://localhost:6379").await?;
let queue = JobQueue::new(backend);
```

### Postgres Backend

```rust
use rustapi_jobs::{JobQueue, PostgresBackend};

let backend = PostgresBackend::new("postgres://localhost/jobs").await?;
let queue = JobQueue::new(backend);
```

---

## Streaming Request Bodies

Handle large uploads efficiently without buffering.

```rust
use rustapi_rs::prelude::*;
use rustapi_core::stream::StreamBody;

#[rustapi_rs::post("/upload")]
async fn upload(body: StreamBody) -> Result<Json<UploadResult>, ApiError> {
    let mut total_size = 0;
    
    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        total_size += chunk.len();
        // Process chunk without holding entire body in memory
    }
    
    Ok(Json(UploadResult { size: total_size }))
}
```

---

## Audit Logging

Track user actions for compliance (GDPR, SOC2).

```rust
use rustapi_extras::audit::{AuditStore, MemoryStore, AuditEvent};

// Create audit store
let store = MemoryStore::new();

// Log an event
store.log(AuditEvent::new("user.login")
    .user_id("user-123")
    .ip_address("192.168.1.1")
    .metadata(json!({ "browser": "Chrome" }))
).await?;

// Query events
let events = store.query()
    .user_id("user-123")
    .action("user.*")
    .since(yesterday)
    .execute()
    .await?;
```

---

## Performance Tips

### 1. Use `simd-json` (when available)

```toml
rustapi-rs = { version = "0.1.4", features = ["simd-json"] }
```

2-4x faster JSON parsing.

### 2. Pre-allocate State

```rust
// Good: Single allocation
let db = Arc::new(RwLock::new(Vec::with_capacity(1000)));

// Avoid: Growing allocations
let db = Arc::new(RwLock::new(Vec::new()));
```

### 3. Use `&'static str` for Static Responses

```rust
// Faster (no allocation)
async fn health() -> &'static str {
    "OK"
}

// Slower (allocates)
async fn health() -> String {
    "OK".to_string()
}
```

### 4. Batch Database Operations

```rust
// Good: Single query
let users = db.query("SELECT * FROM users WHERE id IN ($1)", &[ids]).await;

// Avoid: N queries
for id in ids {
    let user = db.query("SELECT * FROM users WHERE id = $1", &[id]).await;
}
```

---

## Security Best Practices

1. **Always validate input** — Use `ValidatedJson<T>`
2. **Set body limits** — Prevent DoS via large payloads
3. **Use HTTPS in production** — Terminate TLS at load balancer
4. **Rotate JWT secrets** — Store in environment variables
5. **Enable rate limiting** — Prevent brute force attacks
6. **Mask errors in production** — Set `RUSTAPI_ENV=production`

---

For more examples, see the [examples](../examples/) directory.
