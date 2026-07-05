# rustapi-validate: The Gatekeeper

Data validation should happen at the edges of your system, before invalid data ever reaches your business logic. `rustapi-validate` provides a robust, unified validation engine supporting both synchronous and asynchronous rules.

## The Unified Validation System

RustAPI (v0.1.15+) introduces a unified validation system that supports:
1. **Legacy Validator**: The classic `validator` crate (via `#[derive(validator::Validate)]`).
2. **V2 Engine**: The new native engine (via `#[derive(rustapi_macros::Validate)]`) which properly supports async usage.
3. **Async Validation**: Database checks, API calls, and other IO-bound validation rules.

## Synchronous Validation

For standard validation rules (length, email, range, regex), use the `Validate` macro.

> [!TIP]
> Use `rustapi_macros::Validate` for new code to unlock async features.

```rust
use rustapi_macros::Validate; // Logic from V2 engine
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct SignupRequest {
    #[validate(length(min = 3, message = "Username too short"))]
    pub username: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(range(min = 18, max = 150))]
    pub age: u8,
}
```

### The `ValidatedJson` Extractor

For synchronous validation, use the `ValidatedJson<T>` extractor.

```rust
use rustapi_rs::prelude::*;

async fn signup(
    ValidatedJson(payload): ValidatedJson<SignupRequest>
) -> impl IntoResponse {
    // payload is guaranteed to be valid here
    process_signup(payload)
}
```

## Asynchronous Validation

When you need to check data against a database (e.g., "is this email unique?") or an external service, use Async Validation.

### Async Rules

The V2 engine supports async rules directly in the struct definition.

```rust
use rustapi_macros::Validate;
use rustapi_validate::v2::{ValidationContext, RuleError};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    // Built-in async rule (requires database integration)
    #[validate(async_unique(table = "users", column = "email"))]
    pub email: String,

    // Custom async function
    #[validate(custom_async = "check_username_availability")]
    pub username: String,
}

// Custom async validator function
async fn check_username_availability(
    username: &String,
    _ctx: &ValidationContext
) -> Result<(), RuleError> {
    if username == "admin" {
        return Err(RuleError::new("reserved", "This username is reserved"));
    }
    // Perform DB check...
    Ok(())
}
```

### The `AsyncValidatedJson` Extractor

For types with async rules, you **must** use `AsyncValidatedJson`.

```rust
use rustapi_rs::prelude::*;

async fn create_user(
    AsyncValidatedJson(payload): AsyncValidatedJson<CreateUserRequest>
) -> impl IntoResponse {
    // payload is valid AND unique in database
    create_user_in_db(payload).await
}
```

## Error Handling

Whether you use synchronous or asynchronous validation, errors are normalized into a standard `ApiError` format (HTTP 422 Unprocessable Entity).

```json
{
  "error": {
    "type": "validation_error",
    "message": "Request validation failed",
    "fields": [
      {
        "field": "email",
        "code": "email",
        "message": "Invalid email format"
      },
      {
        "field": "username",
        "code": "reserved",
        "message": "This username is reserved"
      }
    ]
  },
  "error_id": "err_a1b2..."
}
```

## Backward Compatibility

The system is fully backward compatible. You can continue using `validator::Validate` on your structs, and `ValidatedJson` will accept them automatically via the unified `Validatable` trait.

```rust
// Legacy code still works!
#[derive(validator::Validate)]
struct OldStruct { ... }

async fn handler(ValidatedJson(body): ValidatedJson<OldStruct>) { ... }
```
