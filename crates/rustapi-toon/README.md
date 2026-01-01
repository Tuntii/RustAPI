# rustapi-toon

TOON (Token-Oriented Object Notation) support for RustAPI framework.

## What is TOON?

TOON is a compact, human-readable format designed for passing structured data to Large Language Models (LLMs) with significantly reduced token usage (typically 20-40% savings).

## Quick Example

**JSON (16 tokens, 40 bytes):**
```json
{
  "users": [
    { "id": 1, "name": "Alice" },
    { "id": 2, "name": "Bob" }
  ]
}
```

**TOON (13 tokens, 28 bytes) - 18.75% token savings:**
```
users[2]{id,name}:
  1,Alice
  2,Bob
```

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
rustapi-rs = { version = "0.1", features = ["toon"] }
```

### Toon Extractor

Parse TOON request bodies:

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::toon::Toon;

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

async fn create_user(Toon(user): Toon<CreateUser>) -> impl IntoResponse {
    // user is parsed from TOON format
    Json(user)
}
```

### Toon Response

Return TOON formatted responses:

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::toon::Toon;

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

async fn get_user() -> Toon<User> {
    Toon(User {
        id: 1,
        name: "Alice".to_string(),
    })
}
```

## Content Types

- Request: `application/toon` or `text/toon`
- Response: `application/toon`

## License

MIT OR Apache-2.0
