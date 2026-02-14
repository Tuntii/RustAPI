# AI Integration

RustAPI offers native support for building AI-friendly APIs using the `rustapi-toon` crate. This allows you to serve optimized content for Large Language Models (LLMs) while maintaining standard JSON responses for traditional clients.

## The Problem: Token Costs

LLMs like GPT-4, Claude, and Gemini charge by the **token**. Standard JSON is verbose, containing many structural characters (`"`, `:`, `{`, `}`) that count towards this limit.

**JSON (55 tokens):**
```json
[
  {"id": 1, "role": "admin", "active": true},
  {"id": 2, "role": "user",  "active": true}
]
```

**TOON (32 tokens):**
```
users[2]{id,role,active}:
  1,admin,true
  2,user,true
```

## The Solution: Content Negotiation

RustAPI uses the `Accept` header to decide which format to return.
- `Accept: application/json` -> Returns JSON.
- `Accept: application/toon` -> Returns TOON.
- `Accept: application/llm` (custom) -> Returns TOON.

This is handled automatically by the `LlmResponse<T>` type.

## Dependencies

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["toon"] }
serde = { version = "1.0", features = ["derive"] }
```

## Implementation

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_toon::LlmResponse; // Handles negotiation
use serde::Serialize;

#[derive(Serialize)]
struct User {
    id: u32,
    username: String,
    role: String,
}

// Simple handler returning a list of users
#[rustapi_rs::get("/users")]
async fn get_users() -> LlmResponse<Vec<User>> {
    let users = vec![
        User { id: 1, username: "Alice".into(), role: "admin".into() },
        User { id: 2, username: "Bob".into(), role: "editor".into() },
    ];

    // LlmResponse automatically serializes to JSON or TOON
    LlmResponse(users)
}

#[tokio::main]
async fn main() {
    let app = RustApi::new().route("/users", get(get_users));

    println!("Server running on http://127.0.0.1:3000");
    RustApi::serve("127.0.0.1:3000", app).await.unwrap();
}
```

## Testing

**Standard Browser / Client:**
```bash
curl http://localhost:3000/users
# Returns: [{"id":1,"username":"Alice",...}]
```

**AI Agent / LLM:**
```bash
curl -H "Accept: application/toon" http://localhost:3000/users
# Returns:
# users[2]{id,username,role}:
#   1,Alice,admin
#   2,Bob,editor
```

## Providing Context to AI

When building an MCP (Model Context Protocol) server or simply feeding data to an LLM, use the TOON format to maximize the context window.

```rust,ignore
// Example: Generating a prompt with TOON data
let data = get_system_status().await;
let toon_string = rustapi_toon::to_string(&data).unwrap();

let prompt = format!(
    "Analyze the following system status and report anomalies:\n\n{}",
    toon_string
);

// Send `prompt` to OpenAI API...
```
