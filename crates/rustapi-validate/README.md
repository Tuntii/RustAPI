# rustapi-validate

**Lens**: "The Gatekeeper"  
**Philosophy**: "Data validation should happen at the edges of your system, before invalid data ever reaches your business logic."

Declarative, type-safe request validation for RustAPI.

## Unified Validation System

RustAPI provides a unified validation system that supports:
1. **Legacy Validator**: The classic `validator` crate (via `#[derive(validator::Validate)]`)
2. **V2 Engine**: The new native engine (via `#[derive(rustapi_macros::Validate)]`) with async support
3. **Async Validation**: Database checks, API calls, and other IO-bound validation rules

## Synchronous Validation

```rust
use rustapi_macros::Validate;
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

For database checks (e.g., "is this email unique?"), use Async Validation:

```rust
use rustapi_macros::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(async_unique(table = "users", column = "email"))]
    pub email: String,

    #[validate(custom_async = "check_username_availability")]
    pub username: String,
}
```

### The `AsyncValidatedJson` Extractor

```rust
use rustapi_rs::prelude::*;

async fn create_user(
    AsyncValidatedJson(payload): AsyncValidatedJson<CreateUserRequest>
) -> impl IntoResponse {
    // payload is valid AND unique in database
    create_user_in_db(payload).await
}
```

## Supported Validators
- `email`, `url`, `length`, `range`
- `contains`, `regex`
- `custom` (sync functions)
- `custom_async` (async functions)
- `async_unique` (database uniqueness)
