# Error Handling

Effective error handling is critical for building robust APIs. RustAPI provides a standard `ApiError` type but encourages custom error handling for domain-specific logic.

## The Standard `ApiError`

RustAPI's `ApiError` is designed to be returned directly from handlers. It implements `IntoResponse`, so it is automatically converted to a JSON response.

```rust
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<i32>) -> Result<Json<User>, ApiError> {
    if id < 0 {
        // Returns 400 Bad Request
        return Err(ApiError::bad_request("ID cannot be negative"));
    }

    if id == 99 {
        // Returns 404 Not Found
        return Err(ApiError::not_found("User not found"));
    }

    // Returns 500 Internal Server Error (and logs the details)
    // The client sees "Internal Server Error" without the sensitive details
    // return Err(ApiError::internal("Database connection failed"));

    Ok(Json(User { id, name: "Alice".into() }))
}
```

### Response Format

By default, `ApiError` produces a standard JSON error response:

```json
{
  "error": {
    "code": 404,
    "message": "User not found",
    "id": "req_123abc" // Request ID for tracking (if tracing is enabled)
  }
}
```

## Custom Error Types

For complex applications, you should define your own error enum to represent domain-specific failures. Implement `IntoResponse` to control how these errors are mapped to HTTP responses.

```rust
use rustapi_rs::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("User not found: {0}")]
    UserNotFound(i32),

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Invalid input: {0}")]
    ValidationError(String),
}

// Map AppError to Response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::UserNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::InsufficientFunds => (StatusCode::PAYMENT_REQUIRED, self.to_string()),
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),

            // Mask internal errors in production!
            AppError::DatabaseError(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal system error".to_string())
            }
        };

        let body = Json(serde_json::json!({
            "error": {
                "message": message,
                "code": status.as_u16()
            }
        }));

        (status, body).into_response()
    }
}
```

### Using Custom Errors in Handlers

Now you can return `Result<T, AppError>` from your handlers.

```rust
async fn transfer(
    Json(payload): Json<TransferRequest>
) -> Result<StatusCode, AppError> {
    let user = find_user(payload.user_id).await?; // Returns AppError::UserNotFound

    if user.balance < payload.amount {
        return Err(AppError::InsufficientFunds);
    }

    // ...

    Ok(StatusCode::OK)
}
```

## Best Practices

### 1. Mask Internal Errors
Never expose raw database errors or stack traces to the client. This is a security risk. Log the full error on the server (using `tracing::error!`) and return a generic "Internal Server Error" message to the client.

### 2. Use `thiserror`
The `thiserror` crate is excellent for defining error hierarchies with minimal boilerplate.

### 3. Structured Logging
When an error occurs, ensure you log it with context.

```rust
AppError::DatabaseError(e) => {
    // Log with structured fields
    tracing::error!(
        error.message = %e,
        error.cause = ?e,
        "Database transaction failed"
    );
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error".into())
}
```

### 4. Global Error Handling
For errors that occur outside of handlers (e.g., in extractors or middleware), RustAPI has default handlers. You can customize 404 and 500 pages using `RustApi::handle_error` (if supported by your version) or by ensuring your extractors return your custom error type.
