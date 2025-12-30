# RustAPI Validate

Validation system for the RustAPI framework.

> **Note**: This is an internal crate. You should depend on `rustapi-rs` instead.

## Features

- **Declarative Validation**: Uses `#[derive(Validate)]` on structs.
- **Common Rules**: Email, Length, Range, Regex.
- **Error Formatting**: Provides standardized 422 JSON error responses.

## Usage

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize, Deserialize, Validate, Schema)]
struct User {
    #[validate(email)]
    email: String,

    #[validate(length(min = 3))]
    username: String,
}
```
