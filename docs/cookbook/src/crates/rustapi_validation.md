# rustapi-validate: The Gatekeeper

Data validation should happen at the edges of your system, before invalid data ever reaches your business logic. `rustapi-validate` integrates the `validator` crate directly into RustAPI's extraction flow.

## The `Validate` Trait

First, define your rules using attributes on your struct.

```rust
use rustapi_validate::Validate;
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

## The `ValidatedJson` Extractor

Instead of using the standard `Json<T>`, use `ValidatedJson<T>`.

```rust
use rustapi_validate::ValidatedJson;

async fn signup(
    ValidatedJson(payload): ValidatedJson<SignupRequest>
) -> impl IntoResponse {
    // If we reach here, 'payload' is guaranteed to be valid!
    // No need to check if email includes '@' or age >= 18.
    
    process_signup(payload)
}
```

## Automatic Error Handling

If validation fails, `ValidatedJson` automatically returns a `400 Bad Request` response with a structured JSON error body detailing exactly which fields failed and why.

```json
{
  "error": "Validation Failed",
  "fields": {
    "email": ["Invalid email format"],
    "age": ["Must be at least 18"]
  }
}
```

## Custom Validation logic

You can also write custom validation functions.

```rust
#[derive(Validate)]
struct Request {
    #[validate(custom = "validate_premium_status")]
    code: String,
}

fn validate_premium_status(code: &str) -> Result<(), rustapi_validate::ValidationError> {
    if !code.starts_with("PREMIUM_") {
        return Err(rustapi_validate::ValidationError::new("Invalid premium code"));
    }
    Ok(())
}
```
