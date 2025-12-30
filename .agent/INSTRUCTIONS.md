# RustAPI - Project Instructions

> **Last Updated:** December 30, 2025  
> **Status:** Phase 1 Complete ✅ | Phase 2 In Progress 🔄

---

## Project Overview

**RustAPI** is a FastAPI-inspired web framework for Rust that prioritizes developer experience (DX) while maintaining Rust's performance and memory safety.

### Core Philosophy
- **5-line Hello World** - Minimal boilerplate
- **Type-driven development** - Structs as schemas
- **API surface is ours, engines can change** - All dependencies wrapped
- **Batteries included but modular** - Easy start, escape hatches available

---

## Project Structure

```
RustAPI/
├── Cargo.toml                    # Workspace root
├── README.md                     # PRD + Manifesto (detailed)
├── memories/
│   ├── TASKLIST.md              # Phase-by-phase task list
│   └── rustapi_memory_bank.md   # Technical reference + decisions
├── crates/
│   ├── rustapi-rs/              # Public facade (crates.io name)
│   ├── rustapi-core/            # Router, extractors, responses, server
│   ├── rustapi-macros/          # Proc-macros (#[rustapi::main], etc.)
│   └── rustapi-validate/        # Validation system (validator wrapper)
└── examples/
    └── hello-world/             # Working example ✅
```

---

## Phase Summary

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1: MVP | ✅ Complete | Router, extractors, responses, basic server |
| Phase 2: Validation + OpenAPI | 🔄 In Progress | Validation wrapper, OpenAPI docs, Swagger UI |
| Phase 3: Batteries Included | ⏳ Planned | Middleware, JWT, CORS, rate limiting |
| Phase 4: v1.0 | ⏳ Planned | Polish, tests, documentation, crates.io publish |

---

## Key Architecture Decisions

1. **Crate name:** `rustapi-rs` (for crates.io)
2. **validator crate:** Wrapper in v0.x, custom engine in v1.x
3. **utoipa crate:** Wrapper only, never expose types
4. **MSRV:** Rust 1.75+
5. **Path params:** `{param}` format (converted to `:param` for matchit)

---

## Development Commands

```powershell
# Build entire workspace
cargo build

# Run hello-world example
cargo run -p hello-world

# Run tests
cargo test --workspace

# Test validation crate
cargo test -p rustapi-validate

# Test specific endpoint
Invoke-RestMethod -Uri http://127.0.0.1:8080/
Invoke-RestMethod -Uri http://127.0.0.1:8080/users/42
```

---

## Current Implementation Status

### ✅ Completed (Phase 1)
- **Router:** Radix tree with matchit, path params, method routing
- **Extractors:** `Json<T>`, `Path<T>`, `Query<T>`, `State<T>`, `Body`
- **Responses:** `Json<T>`, `Created<T>`, `NoContent`, `Html<T>`, `Redirect`
- **Errors:** `ApiError`, `Result<T>` type alias
- **Server:** Hyper 1.x, graceful shutdown, tracing

### 🔄 In Progress (Phase 2)
- **Validation:** `rustapi-validate` crate exists with:
  - `Validate` trait (wraps `validator::Validate`)
  - `FieldError`, `ValidationError` types
  - 422 JSON error format
  - Tests passing
- **Pending:**
  - `ValidatedJson<T>` extractor
  - Integration with `rustapi-core`
  - `rustapi-openapi` crate (utoipa wrapper)
  - Swagger UI embedding

---

## Usage Example (Current)

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize)]
struct HelloResponse { message: String }

async fn hello() -> Json<HelloResponse> {
    Json(HelloResponse { message: "Hello!".into() })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::new()
        .route("/", get(hello))
        .run("127.0.0.1:8080")
        .await
}
```

---

## Target Usage (After Phase 2)

```rust
use rustapi_rs::prelude::*;

#[derive(Schema, Validate, Deserialize)]
struct RegisterRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}

#[derive(Schema, Serialize)]
struct UserOut { id: i64, email: String }

async fn register(body: ValidatedJson<RegisterRequest>) -> Result<Created<UserOut>> {
    // body is already validated!
    Ok(Created(UserOut { id: 1, email: body.email.clone() }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::new()
        .route("/register", post(register))
        .docs("/docs")        // Swagger UI
        .openapi("/api.json") // OpenAPI spec
        .run("127.0.0.1:8080")
        .await
}
```

---

## Related Files

| File | Purpose |
|------|---------|
| `README.md` | Full PRD and manifesto |
| `memories/TASKLIST.md` | Detailed phase-by-phase tasks |
| `memories/rustapi_memory_bank.md` | Technical reference |
| `examples/hello-world/` | Working example |
