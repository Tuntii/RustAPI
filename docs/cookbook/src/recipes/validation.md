# Advanced Validation Patterns

While simple validation (length, range, email) is straightforward with `#[derive(Validate)]`, real-world applications often require complex logic, such as cross-field checks, custom business rules, and asynchronous database lookups.

## Custom Synchronous Validators

You can define custom validation logic by writing a function and referencing it with `#[validate(custom = "...")]`.

### Example: Password Strength

```rust
use rustapi_macros::Validate;
use rustapi_validate::ValidationError;

#[derive(Debug, Deserialize, Validate)]
pub struct SignupRequest {
    #[validate(custom = "validate_password_strength")]
    pub password: String,
}

fn validate_password_strength(password: &String) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::new("password_too_short"));
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_number = password.chars().any(|c| c.is_numeric());

    if !has_uppercase || !has_number {
        return Err(ValidationError::new("password_too_weak"));
    }

    Ok(())
}
```

## Cross-Field Validation

Sometimes validation depends on multiple fields (e.g., "start date must be before end date" or "password confirmation must match"). Since the `Validate` macro works on individual fields, cross-field validation is typically done on the struct level.

Currently, `rustapi-validate` focuses on field-level validation. For struct-level checks, you can implement a custom method and call it manually, or use a "virtual" field strategy.

A common pattern is to validate the struct *after* extraction:

```rust
use rustapi_rs::prelude::*;

#[derive(Debug, Deserialize, Validate)]
pub struct DateRange {
    pub start: chrono::NaiveDate,
    pub end: chrono::NaiveDate,
}

impl DateRange {
    fn validate_logical(&self) -> Result<(), ApiError> {
        if self.start > self.end {
             return Err(ApiError::unprocessable_entity(
                 "start_date_after_end_date",
                 "Start date must be before end date"
             ));
        }
        Ok(())
    }
}

async fn create_event(
    ValidatedJson(payload): ValidatedJson<DateRange>
) -> Result<impl IntoResponse, ApiError> {
    // 1. Basic field validation passes automatically

    // 2. Perform cross-field validation
    payload.validate_logical()?;

    Ok(Json("Event created"))
}
```

## Custom Asynchronous Validators

When you need to check an external source (like a database) during validation, use `#[validate(custom_async = "...")]`.

### Example: Unique Email Check

```rust
use rustapi_macros::Validate;
use rustapi_validate::v2::{ValidationContext, RuleError};
use std::sync::Arc;

// Define your application state
struct AppState {
    db: sqlx::PgPool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(custom_async = "check_email_unique")]
    pub email: String,
}

// The async validator receives the value and the validation context
async fn check_email_unique(email: &String, ctx: &ValidationContext) -> Result<(), RuleError> {
    // 1. Retrieve the database connection from the context
    // The context wraps the AppState you provided to the server
    let state = ctx.get::<Arc<AppState>>()
        .ok_or_else(|| RuleError::new("internal", "Database not available"))?;

    // 2. Perform the query
    let exists = sqlx::query_scalar!("SELECT 1 FROM users WHERE email = $1", email)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| RuleError::new("db_error", "Database error"))?
        .is_some();

    if exists {
        return Err(RuleError::new("email_taken", "This email is already registered"));
    }

    Ok(())
}
```

### Registering the Context

For async validation to work, you must ensure your application state is available to the validator. `AsyncValidatedJson` attempts to extract `ValidationContext` from the request state.

Typically, if you use `RustApi::new().state(...)`, the state is automatically available.

```rust
use rustapi_rs::prelude::*;

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState { /* ... */ });

    RustApi::new()
        .state(state) // Injected into ValidationContext automatically
        .route("/users", post(create_user))
        .run("127.0.0.1:8080")
        .await
        .unwrap();
}

async fn create_user(
    AsyncValidatedJson(payload): AsyncValidatedJson<CreateUserRequest>
) -> impl IntoResponse {
    // payload is valid and email is unique
    Json(payload)
}
```

## Customizing Error Messages

You can override default error messages in the attribute:

```rust
#[derive(Validate)]
struct Request {
    #[validate(length(min = 5, message = "Username must be at least 5 characters"))]
    username: String,

    #[validate(email(message = "Please provide a valid email address"))]
    email: String,
}
```

For custom validators, the `ValidationError` or `RuleError` constructor takes a code and a message:

```rust
ValidationError::new("custom_code").with_message("Friendly error message");
RuleError::new("custom_code", "Friendly error message");
```

This structured error format allows frontend clients to display localized or specific error messages based on the error code.
