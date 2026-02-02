# rustapi-toon

**Lens**: "The Diplomat"  
**Philosophy**: "Optimizing for Silicon Intelligence."

Token-Oriented Object Notation (TOON) support for RustAPI.

## What is TOON?

**T**oken-**O**riented **O**bject **N**otation is a format designed to be consumed by Large Language Models (LLMs). It reduces token usage by stripping unnecessary syntax (braces, quotes) while maintaining semantic structure.

## Token Savings

TOON often reduces token count by 30-50% compared to JSON, saving significant costs and context window space when communicating with models like GPT-4 or Gemini.

## Comparison

**JSON (Expensive)**
```json
[
  {"id": 1, "role": "admin", "active": true},
  {"id": 2, "role": "user",  "active": true},
  {"id": 3, "role": "user",  "active": false}
]
```

**TOON (Optimized)**
```
users[3]{id,role,active}:
  1,admin,true
  2,user,true
  3,user,false
```

## Content Negotiation

The `LlmResponse<T>` type automatically negotiates the response format based on the `Accept` header.

```rust
async fn agent_data() -> LlmResponse<Data> {
    // Returns JSON for browsers
    // Returns TOON for AI Agents (using fewer tokens)
}
```

## Usage

RustAPI handles this transparently via content negotiation.

```rust
use rustapi_toon::Toon;

// Accepts explicit TOON or JSON automatically based on Content-Type
#[rustapi_rs::post("/ingest")]
async fn ingest(Toon(data): Toon<Vec<User>>) -> impl IntoResponse {
    // ...
}
```
