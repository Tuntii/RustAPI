# Database Integration

RustAPI is database-agnostic, but **SQLx** is the recommended driver due to its async-first design and compile-time query verification.

This recipe shows how to integrate PostgreSQL/MySQL/SQLite using a global connection pool.

## Dependencies

```toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
dotenvy = "0.15"
```

## 1. Setup Connection Pool

Create the pool once at startup and share it via `State`.

```rust
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

pub struct AppState {
    pub db: sqlx::PgPool,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    // Run migrations (optional but recommended)
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate");

    let state = Arc::new(AppState { db: pool });

    let app = RustApi::new()
        .route("/users", post(create_user))
        .with_state(state);

    RustApi::serve("0.0.0.0:3000", app).await.unwrap();
}
```

## 2. Using the Database in Handlers

Extract the `State` to get access to the pool.

```rust
use rustapi::prelude::*;

#[derive(Deserialize)]
struct CreateUser {
    username: String,
    email: String,
}

#[derive(Serialize)]
struct User {
    id: i32,
    username: String,
    email: String,
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), ApiError> {
    
    // SQLx query macro performs compile-time checking!
    let record = sqlx::query_as!(
        User,
        "INSERT INTO users (username, email) VALUES ($1, $2) RETURNING id, username, email",
        payload.username,
        payload.email
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(record)))
}
```

## 3. Dependency Injection for Testing

To make testing easier, define a trait for your database operations. This allows you to swap the real DB for a mock.

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, username: &str, email: &str) -> anyhow::Result<User>;
}

// Production implementation
pub struct PostgresRepo(sqlx::PgPool);

#[async_trait]
impl UserRepository for PostgresRepo {
    async fn create(&self, username: &str, email: &str) -> anyhow::Result<User> {
        // ... impl ...
    }
}
```

Then update your state to hold the trait object:

```rust
struct AppState {
    // Dyn dispatch allows swapping impls at runtime
    db: Arc<dyn UserRepository>,
}
```

## Error Handling

Don't expose raw SQL errors to users. Map them to your `ApiError` type.

```rust
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ApiError::NotFound("Resource not found".into()),
            _ => {
                // Log the real error internally
                tracing::error!("Database error: {:?}", err);
                // Return generic error to user
                ApiError::InternalServerError
            }
        }
    }
}
```
