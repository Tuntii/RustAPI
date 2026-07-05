# Handlers & Extractors

The **Handler** is the fundamental unit of work in RustAPI. It transforms an incoming HTTP request into an outgoing HTTP response.

Unlike many web frameworks that enforce a strict method signature (e.g., `fn(req: Request, res: Response)`), RustAPI embraces a flexible, type-safe approach powered by Rust's trait system.

## The Philosophy: "Ask for what you need"

In RustAPI, you don't manually parse the request object inside your business logic. Instead, you declare the data you need as function arguments, and the framework's **Extractors** handle the plumbing for you.

If the data cannot be extracted (e.g., missing header, invalid JSON), the request is rejected *before* your handler is ever called. This means your handler logic is guaranteed to operate on valid, type-safe data.

## Anatomy of a Handler

A handler is simply an asynchronous function that takes zero or more **Extractors** as arguments and returns something that implements `IntoResponse`.

```rust
use rustapi_rs::prelude::*;

async fn create_user(
    State(db): State<DbPool>,         // 1. Dependency Injection
    Path(user_id): Path<Uuid>,        // 2. URL Path Parameter
    Json(payload): Json<CreateUser>,  // 3. JSON Request Body
) -> Result<impl IntoResponse, ApiError> {
    
    let user = db.create_user(user_id, payload).await?;
    
    Ok((StatusCode::CREATED, Json(user)))
}
```

### Key Rules
1. **Order Matters (Slightly)**: Extractors that consume the request body (like `Json<T>` or `Multipart`) must be the *last* argument. This is because the request body is a stream that can only be read once.
2. **Async by Default**: Handlers are `async fn`. This allows non-blocking I/O operations (DB calls, external API requests).
3. **Debuggable**: Handlers are just functions. You can unit test them easily.

## Extractors: The `FromRequest` Trait

Extractors are types that implement `FromRequest` (or `FromRequestParts` for headers/query params). They isolate the "HTTP parsing" logic from your "Business" logic.

### Common Build-in Extractors

| Extractor | Source | Example Usage |
|-----------|--------|---------------|
| `Path<T>` | URL Path Segments | `fn get_user(Path(id): Path<u32>)` |
| `Query<T>` | Query String | `fn search(Query(params): Query<SearchFn>)` |
| `Json<T>` | Request Body | `fn update(Json(data): Json<UpdateDto>)` |
| `HeaderMap` | HTTP Headers | `fn headers(headers: HeaderMap)` |
| `State<T>` | Application State | `fn db_op(State(pool): State<PgPool>)` |
| `Extension<T>` | Request-local extensions | `fn logic(Extension(user): Extension<User>)` |

### Custom Extractors

You can create your own extractors to encapsulate repetitive validation or parsing logic. For example, extracting a user ID from a verified JWT:

```rust
pub struct AuthenticatedUser(pub Uuid);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers.get("Authorization")
            .ok_or(ApiError::Unauthorized("Missing token"))?;
        
        let token = auth_header.to_str().map_err(|_| ApiError::Unauthorized("Invalid token"))?;
        let user_id = verify_jwt(token)?; // Your verification logic
        
        Ok(AuthenticatedUser(user_id))
    }
}

// Usage in handler: cleaner and reusable!
async fn profile(AuthenticatedUser(uid): AuthenticatedUser) -> impl IntoResponse {
    format!("User ID: {}", uid)
}
```

## Responses: The `IntoResponse` Trait

A handler can return any type that implements `IntoResponse`. RustAPI provides implementations for many common types:

- `StatusCode` (e.g., return `200 OK` or `404 Not Found`)
- `Json<T>` (serializes struct to JSON)
- `String` / `&str` (plain text response)
- `Vec<u8>` / `Bytes` (binary data)
- `HeaderMap` (response headers)
- `Html<String>` (HTML content)

### Tuple Responses
You can combine types using tuples to set status codes and headers along with the body:

```rust
// Returns 201 Created + JSON Body
async fn create() -> (StatusCode, Json<User>) {
    (StatusCode::CREATED, Json(user))
}

// Returns Custom Header + Plain Text
async fn custom() -> (HeaderMap, &'static str) {
    let mut headers = HeaderMap::new();
    headers.insert("X-Custom", "Value".parse().unwrap());
    (headers, "Response with headers")
}
```

### Error Handling

Handlers often return `Result<T, E>`. If the handler returns `Ok(T)`, the `T` is converted to a response. If it returns `Err(E)`, the `E` is converted to a response.

This effectively means your `Error` type must implement `IntoResponse`.

```rust
// Recommended pattern: Centralized API Error enum
pub enum ApiError {
    NotFound(String),
    InternalServerError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::InternalServerError => (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong".to_string()),
        };
        
        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

## Best Practices

1. **Keep Handlers Thin**: Move complex business logic to "Service" structs or domain modules. Handlers should focus on HTTP translation (decoding request -> calling service -> encoding response).
2. **Use `State` for Dependencies**: Avoid global variables. Pass DB pools and config via `State`.
3. **Parse Early**: Use specific types in `Json<T>` structs rather than `serde_json::Value` to leverage the type system for validation.
