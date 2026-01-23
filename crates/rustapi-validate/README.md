# RustAPI Validate

**Declarative, type-safe request validation.**

This crate integrates the `validator` library deeply into the RustAPI extractor system.

## The Problem
Manually checking `if email.contains("@")` in every handler is tedious and error-prone.

## The Solution
Define rules on your structs. RustAPI automatically runs them and returns `422 Unprocessable Entity` if constraints are violated.

## Usage

```rust
use rustapi_rs::prelude::*;

#[derive(Deserialize, Validate, Schema)]
pub struct UserSignup {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password too short"))]
    pub password: String,

    #[validate(range(min = 18, max = 100))]
    pub age: u8,
}

// If execution enters this function, `body` is GUARANTEED to be valid.
#[post("/signup")]
async fn signup(Json(body): Json<UserSignup>) -> impl Responder {
    // ...
}
```

## Supported Validators
- `email`
- `url`
- `length`
- `range`
- `custom` (use your own functions)
- `custom_async` (async custom functions)
- `contains`
- `regex`
