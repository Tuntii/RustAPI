# Getting Started with RustAPI

> Build your first API in under 5 minutes.

---

## Prerequisites

- Rust 1.75 or later
- Cargo (comes with Rust)

```bash
# Check your Rust version
rustc --version
```

---

## Installation

Add RustAPI to your `Cargo.toml`:

```toml
[dependencies]
rustapi-rs = "0.1.300"
```

Or with specific features:

```toml
[dependencies]
rustapi-rs = { version = "0.1.300", features = ["jwt", "cors", "toon", "ws", "view"] }
```

### Available Features

| Feature | Description |
|---------|-------------|
| `swagger-ui` | Swagger UI at `/docs` (enabled by default) |
| `jwt` | JWT authentication |
| `cors` | CORS middleware |
| `rate-limit` | IP-based rate limiting |
| `toon` | LLM-optimized TOON format |
| `ws` | WebSocket support |
| `view` | Template engine (Tera) |
| `simd-json` | 2-4x faster JSON parsing |
| `audit` | GDPR/SOC2 audit logging |
| `full` | All features |

---

## Hello World

Create a new project:

```bash
cargo new hello-rustapi
cd hello-rustapi
```

Add the dependency:

```bash
cargo add rustapi-rs
```

Edit `src/main.rs`:

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize, Schema)]
struct Message {
    greeting: String,
}

#[rustapi_rs::get("/")]
async fn hello() -> Json<Message> {
    Json(Message {
        greeting: "Hello, World!".into(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::auto().run("127.0.0.1:8080").await
}
```

Run it:

```bash
cargo run
```

Test it:

```bash
# Terminal / PowerShell
curl http://localhost:8080/

# Or open in browser
# http://localhost:8080/docs  ← Swagger UI
```

---

## Understanding the Code

### 1. The Prelude Import

```rust
use rustapi_rs::prelude::*;
```

This imports everything you need:
- `RustApi` — Application builder
- `Json`, `Path`, `Query`, `State` — Extractors
- `Serialize`, `Deserialize` — Serde macros
- `Schema` — OpenAPI schema generation
- `get`, `post`, `put`, `patch`, `delete` — Route functions

### 2. Schema Derivation

```rust
#[derive(Serialize, Schema)]
struct Message {
    greeting: String,
}
```

- `Serialize` — Enables JSON serialization
- `Schema` — Generates OpenAPI documentation automatically

### 3. Route Macro

```rust
#[rustapi_rs::get("/")]
async fn hello() -> Json<Message> { ... }
```

The `#[rustapi_rs::get]` macro:
- Registers the route at compile time
- Generates OpenAPI documentation
- Works with `RustApi::auto()` for zero-config routing

### 4. Auto Configuration

```rust
RustApi::auto().run("127.0.0.1:8080").await
```

`RustApi::auto()` automatically:
- Discovers all `#[rustapi_rs::get/post/...]` routes
- Enables Swagger UI at `/docs`
- Enables OpenAPI spec at `/openapi.json`

---

## Adding Parameters

### Path Parameters

```rust
#[derive(Serialize, Schema)]
struct User {
    id: u64,
    name: String,
}

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<u64>) -> Json<User> {
    Json(User {
        id,
        name: format!("User {}", id),
    })
}
```

Test: `curl http://localhost:8080/users/42`

### Query Parameters

```rust
#[derive(Deserialize, Schema)]
struct Pagination {
    page: Option<u32>,
    limit: Option<u32>,
}

#[rustapi_rs::get("/users")]
async fn list_users(Query(params): Query<Pagination>) -> Json<Vec<User>> {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10);
    // Fetch users...
    Json(vec![])
}
```

Test: `curl http://localhost:8080/users?page=2&limit=20`

### Request Body

```rust
#[derive(Deserialize, Schema)]
struct CreateUser {
    name: String,
    email: String,
}

#[rustapi_rs::post("/users")]
async fn create_user(Json(body): Json<CreateUser>) -> Json<User> {
    Json(User {
        id: 1,
        name: body.name,
    })
}
```

Test:
```bash
curl -X POST http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice", "email": "alice@example.com"}'
```

---

## Validation

Add validation to your requests:

```rust
use rustapi_rs::prelude::*;

#[derive(Deserialize, Validate, Schema)]
struct CreateUser {
    #[validate(length(min = 3, max = 50))]
    name: String,
    
    #[validate(email)]
    email: String,
    
    #[validate(range(min = 0, max = 150))]
    age: u8,
}

#[rustapi_rs::post("/users")]
async fn create_user(ValidatedJson(body): ValidatedJson<CreateUser>) -> Json<User> {
    // `body` is guaranteed valid at this point
    Json(User { id: 1, name: body.name })
}
```

Invalid requests return 422 with detailed errors:

```json
{
    "status": 422,
    "error_type": "validation_error",
    "message": "Request validation failed",
    "fields": [
        {"field": "email", "message": "Invalid email format"},
        {"field": "age", "message": "Must be between 0 and 150"}
    ]
}
```

### Available Validators

| Validator | Usage |
|-----------|-------|
| `email` | `#[validate(email)]` |
| `length` | `#[validate(length(min = 1, max = 100))]` |
| `range` | `#[validate(range(min = 0, max = 999))]` |
| `regex` | `#[validate(regex(path = "PHONE_REGEX"))]` |
| `url` | `#[validate(url)]` |
| `custom` | `#[validate(custom(function = "my_validator"))]` |

---

## Application State

Share data across handlers:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

type Db = Arc<RwLock<Vec<User>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db: Db = Arc::new(RwLock::new(vec![]));
    
    RustApi::new()
        .state(db)
        .route("/users", get(list_users))
        .route("/users", post(create_user))
        .run("127.0.0.1:8080")
        .await
}

async fn list_users(State(db): State<Db>) -> Json<Vec<User>> {
    let users = db.read().await;
    Json(users.clone())
}

async fn create_user(
    State(db): State<Db>,
    Json(body): Json<CreateUser>,
) -> Json<User> {
    let mut users = db.write().await;
    let user = User { id: users.len() as u64 + 1, name: body.name };
    users.push(user.clone());
    Json(user)
}
```

---

## Error Handling

RustAPI provides the `Result` type alias:

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<u64>) -> Result<Json<User>> {
    if id == 0 {
        return Err(ApiError::bad_request("ID cannot be zero"));
    }
    
    let user = find_user(id)
        .ok_or_else(|| ApiError::not_found(format!("User {} not found", id)))?;
    
    Ok(Json(user))
}
```

### Built-in Error Helpers

```rust
ApiError::bad_request("message")     // 400
ApiError::unauthorized("message")     // 401
ApiError::forbidden("message")        // 403
ApiError::not_found("message")        // 404
ApiError::conflict("message")         // 409
ApiError::unprocessable("message")    // 422
ApiError::internal("message")         // 500
```

---

## Middleware

### CORS

```toml
rustapi-rs = { version = "0.1.300", features = ["cors"] }
```

```rust
use rustapi_rs::middleware::CorsLayer;

RustApi::new()
    .layer(CorsLayer::permissive())  // Allow all origins
    // Or configure:
    // .layer(CorsLayer::new()
    //     .allow_origin("https://example.com")
    //     .allow_methods(["GET", "POST"])
    //     .allow_headers(["Content-Type"]))
    .route("/api/data", get(data))
    .run("0.0.0.0:8080")
    .await
```

### JWT Authentication

```toml
rustapi-rs = { version = "0.1.300", features = ["jwt"] }
```

```rust
use rustapi_rs::middleware::JwtLayer;
use rustapi_rs::extract::AuthUser;

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
}

RustApi::new()
    .layer(JwtLayer::new("your-secret-key").skip_paths(["/login", "/health"]))
    .route("/protected", get(protected))
    .route("/login", post(login))
    .run("0.0.0.0:8080")
    .await

async fn protected(user: AuthUser<Claims>) -> Json<Response> {
    Json(Response {
        message: format!("Hello, {}", user.claims.sub)
    })
}
```

### Rate Limiting

```toml
rustapi-rs = { version = "0.1.300", features = ["rate-limit"] }
```

```rust
use rustapi_rs::middleware::RateLimitLayer;

RustApi::new()
    .layer(RateLimitLayer::new(100, Duration::from_secs(60)))  // 100 req/min
    .route("/api", get(handler))
    .run("0.0.0.0:8080")
    .await
```

---

## TOON Format (LLM Optimization)

```toml
rustapi-rs = { version = "0.1.300", features = ["toon"] }
```

```rust
use rustapi_rs::toon::{Toon, LlmResponse, AcceptHeader};

// Direct TOON response
#[rustapi_rs::get("/ai/users")]
async fn ai_users() -> Toon<UsersResponse> {
    Toon(get_users())
}

// Content negotiation based on Accept header
#[rustapi_rs::get("/users")]
async fn users(accept: AcceptHeader) -> LlmResponse<UsersResponse> {
    LlmResponse::new(get_users(), accept.preferred)
}
```

Response includes token counting headers:
- `X-Token-Count-JSON`: Token count for JSON format
- `X-Token-Count-TOON`: Token count for TOON format
- `X-Token-Savings`: Percentage saved (e.g., "57.8%")

---

## WebSocket Support

Real-time bidirectional communication:

```toml
rustapi-rs = { version = "0.1.300", features = ["ws"] }
```

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
            Message::Close(_) => break,
            _ => {}
        }
    }
}
```

Test with `websocat`:
```bash
websocat ws://localhost:8080/ws
```

---

## Template Engine

Server-side HTML rendering with Tera:

```toml
rustapi-rs = { version = "0.1.300", features = ["view"] }
```

Create a template file `templates/index.html`:
```html
<!DOCTYPE html>
<html>
<head><title>{{ title }}</title></head>
<body>
    <h1>Hello, {{ name }}!</h1>
</body>
</html>
```

Use in your handler:
```rust
use rustapi_rs::view::{Templates, View, TemplatesConfig};

#[tokio::main]
async fn main() {
    let templates = Templates::new(TemplatesConfig {
        directory: "templates".into(),
        extension: "html".into(),
    }).unwrap();
    
    RustApi::new()
        .state(templates)
        .route("/", get(home))
        .run("0.0.0.0:8080")
        .await
}

#[derive(Serialize)]
struct HomeData {
    title: String,
    name: String,
}

#[rustapi_rs::get("/")]
async fn home(templates: Templates) -> View<HomeData> {
    View::new(&templates, "index.html", HomeData {
        title: "Welcome".into(),
        name: "World".into(),
    })
}
```

---

## CLI Tool

Scaffold new RustAPI projects quickly:

```bash
# Install the CLI
cargo install cargo-rustapi

# Create a new project
cargo rustapi new my-api

# Interactive mode with template selection
cargo rustapi new my-api --interactive

# Run with hot-reload (auto-restart on file changes)
cargo rustapi watch

# Add features or dependencies
cargo rustapi add cors jwt

# Check environment health
cargo rustapi doctor
```

Available templates:
- `minimal` — Basic RustAPI setup
- `api` — REST API with CRUD operations
- `web` — Full web app with templates and WebSocket
- `full` — Everything included

---

## Testing

```rust
use rustapi_rs::test::TestClient;

#[tokio::test]
async fn test_hello() {
    let app = RustApi::new()
        .route("/", get(hello));
    
    let client = TestClient::new(app);
    
    let res = client.get("/").send().await;
    assert_eq!(res.status(), 200);
    
    let body: Message = res.json().await;
    assert_eq!(body.greeting, "Hello, World!");
}

#[tokio::test]
async fn test_create_user() {
    let app = RustApi::new()
        .route("/users", post(create_user));
    
    let client = TestClient::new(app);
    
    let res = client
        .post("/users")
        .json(&CreateUser { name: "Alice".into(), email: "alice@test.com".into() })
        .send()
        .await;
    
    assert_eq!(res.status(), 200);
}
```

---

## Next Steps

- 📖 [Philosophy](PHILOSOPHY.md) — Understand our design principles
- 🏗️ [Architecture](ARCHITECTURE.md) — Deep dive into internals
- 📚 [Features](FEATURES.md) — Complete feature documentation
- 💡 [Examples](../examples/) — Real-world examples

---

## Common Patterns

### Health Check

```rust
#[rustapi_rs::get("/health")]
async fn health() -> &'static str {
    "OK"
}
```

### Multiple Response Types

```rust
#[rustapi_rs::get("/data")]
async fn data() -> Result<Json<Data>> {
    match fetch_data().await {
        Ok(data) => Ok(Json(data)),
        Err(_) => Err(ApiError::not_found("Data not found")),
    }
}
```

### Custom Status Codes

```rust
use rustapi_rs::response::Created;

#[rustapi_rs::post("/users")]
async fn create_user(Json(body): Json<CreateUser>) -> Created<User> {
    let user = User { id: 1, name: body.name };
    Created(user)  // Returns 201 Created
}
```

---

## Troubleshooting

### "Route not found" for macro-decorated handlers

Make sure you're using `RustApi::auto()`:

```rust
// ✅ Correct
RustApi::auto().run("0.0.0.0:8080").await

// ❌ Won't find macro routes
RustApi::new().run("0.0.0.0:8080").await
```

### Compilation errors with extractors

Ensure your types implement required traits:

```rust
// ✅ For request bodies
#[derive(Deserialize)]
struct RequestBody { ... }

// ✅ For responses
#[derive(Serialize)]
struct ResponseBody { ... }

// ✅ For OpenAPI docs
#[derive(Schema)]
struct AnyBody { ... }
```

### Swagger UI not showing

Check that the `swagger-ui` feature is enabled (it's on by default):

```toml
rustapi-rs = { version = "0.1.300", features = ["swagger-ui"] }
```

### CLI Commands Not Working

Use the new `doctor` command to diagnose:

```bash
cargo rustapi doctor
```

---

Happy coding! 🦀
