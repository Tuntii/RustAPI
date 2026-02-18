# Database Integration

RustAPI is database-agnostic, but **SQLx** is the recommended driver due to its async-first design and compile-time query verification.

This recipe shows how to integrate PostgreSQL/MySQL/SQLite using a global connection pool.

## Dependencies

```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
dotenvy = "0.15"
# Make sure async-trait is enabled if using traits for repositories
async-trait = "0.1"
```

## 1. Setup Connection Pool

Create the pool once at startup and share it via `State`.

```rust
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use rustapi_rs::prelude::*;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create a connection pool with proper configuration
    let pool = PgPoolOptions::new()
        .max_connections(50) // Adjust based on your DB capabilities
        .min_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    // Run migrations (optional but recommended)
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate");

    let state = AppState { db: pool };

    RustApi::new()
        .state(state) // Inject state
        .route("/users", post(create_user))
        .run("0.0.0.0:3000")
        .await
        .unwrap();
}
```

## 2. Using the Database in Handlers

Extract the `State` to get access to the pool.

```rust
use rustapi_rs::prelude::*;

#[derive(Deserialize, Schema)]
struct CreateUser {
    username: String,
    email: String,
}

#[derive(Serialize, Schema)]
struct User {
    id: i32,
    username: String,
    email: String,
}

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<User>, ApiError> {
    
    // SQLx query macro performs compile-time checking!
    let record = sqlx::query_as!(
        User,
        "INSERT INTO users (username, email) VALUES ($1, $2) RETURNING id, username, email",
        payload.username,
        payload.email
    )
    .fetch_one(&state.db)
    .await
    .map_err(map_sql_error)?;

    Ok(Json(record))
}

fn map_sql_error(err: sqlx::Error) -> ApiError {
    match err {
        sqlx::Error::RowNotFound => ApiError::not_found("Resource not found"),
        sqlx::Error::Database(db_err) => {
            // Check for unique constraint violation (Postgres error 23505)
            if db_err.code().as_deref() == Some("23505") {
                 return ApiError::conflict("Resource already exists");
            }
            ApiError::internal(db_err.message())
        }
        _ => {
            tracing::error!("Database error: {:?}", err);
            ApiError::internal("Internal Server Error")
        }
    }
}
```

## 3. Transactions

For operations that modify multiple tables, use transactions to ensure data integrity.

```rust
async fn transfer_funds(
    State(state): State<AppState>,
    Json(payload): Json<TransferRequest>,
) -> Result<Json<TransferResult>, ApiError> {

    // Start a transaction
    let mut tx = state.db.begin().await.map_err(map_sql_error)?;

    // Deduct from sender
    let sender = sqlx::query!("UPDATE accounts SET balance = balance - $1 WHERE id = $2 RETURNING balance", payload.amount, payload.from_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sql_error)?;

    if sender.balance < 0 {
        // Rollback implicitly by returning error (tx dropped)
        return Err(ApiError::bad_request("Insufficient funds"));
    }

    // Add to receiver
    sqlx::query!("UPDATE accounts SET balance = balance + $1 WHERE id = $2", payload.amount, payload.to_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sql_error)?;

    // Commit transaction
    tx.commit().await.map_err(map_sql_error)?;

    Ok(Json(TransferResult { success: true }))
}
```

## 4. Repository Pattern (Advanced)

To isolate DB logic and make testing easier, define a trait.

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
        let user = sqlx::query_as!(
            User,
            "INSERT INTO users (username, email) VALUES ($1, $2) RETURNING id, username, email",
            username,
            email
        )
        .fetch_one(&self.0)
        .await?;
        Ok(user)
    }
}

// In your AppState
#[derive(Clone)]
struct AppState {
    users: Arc<dyn UserRepository>,
}
```

This allows you to implement a `MockUserRepository` for unit tests without spinning up a real database.

```rust
struct MockUserRepository;

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn create(&self, _u: &str, _e: &str) -> anyhow::Result<User> {
        Ok(User { id: 1, username: "mock".into(), email: "mock@test.com".into() })
    }
}
```
