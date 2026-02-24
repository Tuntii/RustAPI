# Recipe: Custom Validation

Sometimes the built-in validators (`email`, `length`, `range`) aren't enough. You might need to check if a username is taken, if a coupon code is valid, or if two fields match.

## Problem
You need to enforce business rules that require custom logic or external data lookups (like a database).

## Solution: Synchronous Custom Validators

For logic that doesn't require async operations (like comparing two fields or checking a format), use `custom`.

### 1. Define the Validator Function
The function must have the signature `fn(&T) -> Result<(), ValidationError>`.

```rust
use rustapi_validate::ValidationError;

fn validate_no_spaces(username: &str) -> Result<(), ValidationError> {
    if username.contains(' ') {
        return Err(ValidationError::new("username_spaces"));
    }
    Ok(())
}
```

### 2. Apply it to the Struct

```rust
use rustapi_macros::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct SignupRequest {
    #[validate(custom = "validate_no_spaces")]
    pub username: String,
}
```

## Solution: Asynchronous Custom Validators

For logic that requires I/O (like database checks), use `custom_async`.

### 1. Define the Async Validator Function
The signature must be `async fn(&T, &ValidationContext) -> Result<(), RuleError>`.

```rust
use rustapi_validate::v2::{RuleError, ValidationContext};

async fn validate_username_available(
    username: &String,
    _ctx: &ValidationContext,
) -> Result<(), RuleError> {
    // Simulate a DB call
    // In real code, you would access the DB via _ctx
    if username == "admin" {
        return Err(RuleError::new("unique", "Username is already taken"));
    }
    Ok(())
}
```

### 2. Apply it to the Struct

```rust
use rustapi_macros::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(custom_async = "validate_username_available")]
    pub username: String,
}
```

### 3. Using ValidationContext
To pass dependencies (like a DB pool) to your validator, use the `ValidationContext`.

First, implement a custom `DatabaseValidator` or stick your own types into the context if supported (currently `ValidationContext` is optimized for the built-in `DatabaseValidator` trait, but you can use `custom_async` to bridge gaps).

If you are using `AsyncValidatedJson`, the extractor automatically looks for a `ValidationContext` in the request state.

## Full Example

```rust
use rustapi_rs::prelude::*;
use rustapi_validate::v2::{RuleError, ValidationContext};
use serde::Deserialize;

// 1. The DTO
#[derive(Debug, Deserialize, Validate)]
pub struct Product {
    #[validate(length(min = 3))]
    pub name: String,

    #[validate(custom_async = "validate_sku_format")]
    pub sku: String,
}

// 2. The Custom Validator
async fn validate_sku_format(sku: &String, _ctx: &ValidationContext) -> Result<(), RuleError> {
    if !sku.starts_with("SKU-") {
        return Err(RuleError::new("format", "SKU must start with SKU-"));
    }
    Ok(())
}

// 3. The Handler
async fn create_product(
    AsyncValidatedJson(product): AsyncValidatedJson<Product>
) -> impl IntoResponse {
    Json(product)
}
```

## Discussion

- **Performance**: Async validators are only run if synchronous validators pass.
- **Context**: The `ValidationContext` is key for dependency injection. It allows your validators to remain pure and testable while still having access to the outside world.
- **Error Messages**: You can override the default error message in the attribute: `#[validate(custom = "my_func", message = "Bad value")]`.
