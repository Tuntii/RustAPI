# Error Handling

RustAPI ships with a structured `ApiError` type and a consistent wire format for error responses. The trick is not just returning errors, but returning the **right** error to the client while keeping internal details out of production responses.

## Problem

Without a clear error strategy, handlers tend to mix:

- business errors,
- validation errors,
- infrastructure errors, and
- internal debugging details.

That usually leads to noisy handlers and accidental leakage of sensitive information.

## Solution

Use `ApiError` at the HTTP boundary and convert your domain/application errors into it.

### Basic handler pattern

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize, Schema)]
struct UserDto {
    id: u64,
    email: String,
}

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<u64>) -> Result<Json<UserDto>> {
    if id == 0 {
        return Err(ApiError::bad_request("id must be greater than zero"));
    }

    let user = find_user(id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("User {} not found", id)))?;

    Ok(Json(user))
}

async fn find_user(_id: u64) -> Result<Option<UserDto>> {
    Ok(None)
}
```

### Mapping application errors into `ApiError`

```rust
use rustapi_rs::prelude::*;

#[derive(Debug)]
enum AppError {
    UserNotFound(u64),
    DuplicateEmail,
    Storage(std::io::Error),
}

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::UserNotFound(id) => {
                ApiError::not_found(format!("User {} not found", id))
            }
            AppError::DuplicateEmail => {
                ApiError::conflict("A user with that email already exists")
            }
            AppError::Storage(source) => {
                ApiError::internal("Storage error").with_internal(source.to_string())
            }
        }
    }
}

#[derive(Serialize, Schema)]
struct UserDto {
    id: u64,
    email: String,
}

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<u64>) -> Result<Json<UserDto>> {
    let user = load_user(id).await?;
    Ok(Json(user))
}

async fn load_user(id: u64) -> std::result::Result<UserDto, AppError> {
    if id == 42 {
        return Err(AppError::UserNotFound(id));
    }

    Ok(UserDto {
        id,
        email: "demo@example.com".into(),
    })
}
```

### Validation errors are already normalized

```rust
use rustapi_rs::prelude::*;

#[derive(Deserialize, Validate, Schema)]
struct CreateUser {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}

#[rustapi_rs::post("/users")]
async fn create_user(ValidatedJson(body): ValidatedJson<CreateUser>) -> Result<StatusCode> {
    let _ = body;
    Ok(StatusCode::CREATED)
}
```

If validation fails, RustAPI returns `422 Unprocessable Entity` automatically.

## Error response shape

RustAPI serializes errors as JSON like this:

```json
{
  "error": {
    "type": "not_found",
    "message": "User 42 not found"
  },
  "error_id": "err_a1b2c3d4e5f6..."
}
```

Validation errors add `fields`:

```json
{
  "error": {
    "type": "validation_error",
    "message": "Request validation failed",
    "fields": [
      {
        "field": "email",
        "code": "email",
        "message": "must be a valid email"
      }
    ]
  },
  "error_id": "err_a1b2c3d4e5f6..."
}
```

## Discussion

### Use 4xx for client-facing corrections

Good candidates for direct client messages:

- `bad_request`
- `unauthorized`
- `forbidden`
- `not_found`
- `conflict`
- validation failures

### Use 5xx for internal failures

For infrastructure or unexpected failures, prefer `ApiError::internal(...)` and attach private details with `.with_internal(...)`.

That gives operators useful logs without sending those internals to clients.

### Production masking

When `RUSTAPI_ENV=production`, server-side error messages are masked automatically.

Example:

- development 500 message: `Storage error`
- production 500 message: `An internal error occurred`

Validation field details still remain visible.

### Error correlation

Every response includes an `error_id`. Use it to correlate:

- client reports,
- server logs,
- trace/span data,
- audit or replay workflows.

### SQLx integration

When the SQLx feature is enabled, `sqlx::Error` converts into `ApiError` automatically. That means `?` works naturally in many handlers while still mapping common database failures to sensible HTTP responses.

## Testing

Manual checks:

```bash
curl -i http://127.0.0.1:8080/users/0
curl -i http://127.0.0.1:8080/users/42
curl -i -X POST http://127.0.0.1:8080/users -H "content-type: application/json" --data "{\"email\":\"bad\",\"password\":\"123\"}"
```

What to verify:

- `400` returns a `bad_request` error body
- `404` returns a `not_found` error body
- `422` returns `fields` entries
- every error payload contains `error_id`

## Related reading

- [Getting Started](../../../GETTING_STARTED.md#error-handling)
- [Database Integration](db_integration.md)
- [Troubleshooting](../troubleshooting.md)