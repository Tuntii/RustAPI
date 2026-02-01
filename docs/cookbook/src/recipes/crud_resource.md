# Creating Resources

**Problem**: You need to add a new "Resource" (like Users, Products, or Posts) to your API with standard CRUD operations.

## Solution

Create a new module `src/handlers/users.rs`:

```rust
use rustapi_rs::prelude::*;

#[derive(Serialize, Deserialize, Schema, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
}

#[derive(Deserialize, Schema)]
pub struct CreateUser {
    pub name: String,
}

#[rustapi::get("/users")]
pub async fn list() -> Json<Vec<User>> {
    Json(vec![]) // Fetch from DB in real app
}

#[rustapi::post("/users")]
pub async fn create(Json(payload): Json<CreateUser>) -> impl IntoResponse {
    let user = User { id: 1, name: payload.name };
    (StatusCode::CREATED, Json(user))
}
```

Then in `main.rs`, simply use `RustApi::auto()`:

```rust
use rustapi_rs::prelude::*;

mod handlers; // Make sure the module is part of the compilation unit!

#[rustapi::main]
async fn main() -> Result<()> {
    // RustAPI automatically discovers all routes decorated with macros
    RustApi::auto()
        .run("127.0.0.1:8080")
        .await
}
```

## Discussion

RustAPI uses **distributed slices** (via `linkme`) to automatically register routes decorated with `#[rustapi::get]`, `#[rustapi::post]`, etc. This means you don't need to manually import or mount every single handler in your `main` function.

Just ensure your handler modules are reachable (e.g., via `mod handlers;`), and the framework handles the rest. This encourages a clean, Domain-Driven Design (DDD) structure where resources are self-contained.
