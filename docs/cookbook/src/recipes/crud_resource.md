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

Then register it in `main.rs`:

```rust
RustApi::new()
    .mount(handlers::users::list)
    .mount(handlers::users::create)
```

## Discussion

Using `#[rustapi::mount]` (if available) or manual routing keeps your `main.rs` clean. Organizing handlers by resource (domain-driven design) scales better than organizing by HTTP method.
