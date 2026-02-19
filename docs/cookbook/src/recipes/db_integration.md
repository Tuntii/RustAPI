# Database Integration

RustAPI is database-agnostic, but **SQLx** is the recommended driver due to its async-first design and compile-time query verification.

This recipe shows how to integrate PostgreSQL/MySQL/SQLite using a global connection pool with best practices for production.

## Dependencies

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["sqlx"] } # Enable SQLx error conversion
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid"] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
dotenvy = "0.15"
```

## 1. Setup Connection Pool

Create the pool once at startup and share it via `State`. Configure pool limits appropriately.

```rust
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create a connection pool with production settings
    let pool = PgPoolOptions::new()
        .max_connections(50) // Adjust based on DB limits
        .min_connections(5)  // Keep some idle connections ready
        .acquire_timeout(Duration::from_secs(5)) // Fail fast if DB is overloaded
        .idle_timeout(Duration::from_secs(300))  // Close idle connections
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    // Run migrations (optional but recommended)
    // Note: requires `sqlx-cli` or `sqlx` migrate feature
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate");

    let state = AppState { db: pool };

    RustApi::new()
        .state(state)
        .route("/users", post(create_user))
        .run("0.0.0.0:3000")
        .await
}
```

## 2. Using the Database in Handlers

Extract the `State` to get access to the pool.

```rust
use rustapi_rs::prelude::*;

#[derive(Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 3))]
    username: String,
    #[validate(email)]
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
    ValidatedJson(payload): ValidatedJson<CreateUser>,
) -> Result<(StatusCode, Json<User>), ApiError> {
    
    // SQLx query macro performs compile-time checking!
    // The query is checked against your running database during compilation.
    let record = sqlx::query_as!(
        User,
        "INSERT INTO users (username, email) VALUES ($1, $2) RETURNING id, username, email",
        payload.username,
        payload.email
    )
    .fetch_one(&state.db)
    .await
    // Map sqlx::Error to ApiError (feature = "sqlx" handles this automatically)
    .map_err(ApiError::from)?;

    Ok((StatusCode::CREATED, Json(record)))
}
```

## 3. Transactions

For operations involving multiple queries, use a transaction to ensure atomicity.

```rust
async fn transfer_credits(
    State(state): State<AppState>,
    Json(payload): Json<TransferRequest>,
) -> Result<StatusCode, ApiError> {
    // Start a transaction
    let mut tx = state.db.begin().await.map_err(ApiError::from)?;

    // Deduct from sender
    let updated = sqlx::query!(
        "UPDATE accounts SET balance = balance - $1 WHERE id = $2 RETURNING balance",
        payload.amount,
        payload.sender_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    // Check balance
    if let Some(record) = updated {
        if record.balance < 0 {
            // Rollback is automatic on drop, but explicit rollback is clearer
            tx.rollback().await.map_err(ApiError::from)?;
            return Err(ApiError::bad_request("Insufficient funds"));
        }
    } else {
        return Err(ApiError::not_found("Sender not found"));
    }

    // Add to receiver
    sqlx::query!(
        "UPDATE accounts SET balance = balance + $1 WHERE id = $2",
        payload.amount,
        payload.receiver_id
    )
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    // Commit transaction
    tx.commit().await.map_err(ApiError::from)?;

    Ok(StatusCode::OK)
}
```

## 4. Integration Testing with TestContainers

For testing, use `testcontainers` to spin up a real database instance. This ensures your queries are correct without mocking the database driver.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::{clients, images};
    use rustapi_testing::TestClient;

    #[tokio::test]
    async fn test_create_user() {
        // Start Postgres container
        let docker = clients::Cli::default();
        let pg = docker.run(images::postgres::Postgres::default());
        let port = pg.get_host_port_ipv4(5432);
        let db_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

        // Setup pool
        let pool = PgPoolOptions::new().connect(&db_url).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let state = AppState { db: pool };

        // Create app and client
        let app = RustApi::new().state(state).route("/users", post(create_user));
        let client = TestClient::new(app);

        // Test request
        let response = client.post("/users")
            .json(&serde_json::json!({
                "username": "testuser",
                "email": "test@example.com"
            }))
            .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let user: User = response.json().await;
        assert_eq!(user.username, "testuser");
    }
}
```

## Error Handling

RustAPI provides automatic conversion from `sqlx::Error` to `ApiError` when the `sqlx` feature is enabled.

- `RowNotFound` -> 404 Not Found
- `PoolTimedOut` -> 503 Service Unavailable
- Unique Constraint Violation -> 409 Conflict
- Check Constraint Violation -> 400 Bad Request
- Other errors -> 500 Internal Server Error (masked in production)
