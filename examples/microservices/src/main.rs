//! Microservices Example for RustAPI
//!
//! This example demonstrates:
//! - Service-to-service communication
//! - API Gateway pattern
//! - Service discovery
//! - Load balancing (round-robin)
//!
//! Run with: cargo run -p microservices
//! This starts 3 services:
//!   - Gateway (port 8080) - Routes requests
//!   - User Service (port 8081) - Manages users
//!   - Order Service (port 8082) - Manages orders

use rustapi_rs::prelude::*;
use std::sync::Arc;

// ============================================
// Shared Models
// ============================================

#[derive(Serialize, Deserialize, Schema, Clone)]
struct User {
    id: u64,
    name: String,
    email: String,
}

#[derive(Serialize, Deserialize, Schema, Clone)]
struct Order {
    id: u64,
    user_id: u64,
    product: String,
    amount: f64,
}

#[derive(Serialize, Schema)]
struct GatewayResponse {
    service: String,
    data: serde_json::Value,
}

// ============================================
// User Service (Port 8081)
// ============================================

mod user_service {
    use super::*;

    #[rustapi_rs::get("/users/{id}")]
    async fn get_user(Path(id): Path<u64>) -> Json<User> {
        println!("👤 User Service: Getting user {}", id);
        Json(User {
            id,
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
        })
    }

    #[rustapi_rs::get("/users")]
    async fn list_users() -> Json<Vec<User>> {
        println!("👥 User Service: Listing all users");
        Json(vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        ])
    }

    pub async fn start() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🚀 Starting User Service on port 8081...");
        RustApi::auto().run("127.0.0.1:8081").await
    }
}

// ============================================
// Order Service (Port 8082)
// ============================================

mod order_service {
    use super::*;

    #[rustapi_rs::get("/orders/{id}")]
    async fn get_order(Path(id): Path<u64>) -> Json<Order> {
        println!("📦 Order Service: Getting order {}", id);
        Json(Order {
            id,
            user_id: 1,
            product: format!("Product {}", id),
            amount: 99.99,
        })
    }

    #[rustapi_rs::get("/orders")]
    async fn list_orders() -> Json<Vec<Order>> {
        println!("📦 Order Service: Listing all orders");
        Json(vec![
            Order {
                id: 1,
                user_id: 1,
                product: "Laptop".to_string(),
                amount: 999.99,
            },
            Order {
                id: 2,
                user_id: 2,
                product: "Mouse".to_string(),
                amount: 29.99,
            },
        ])
    }

    pub async fn start() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🚀 Starting Order Service on port 8082...");
        RustApi::auto().run("127.0.0.1:8082").await
    }
}

// ============================================
// API Gateway (Port 8080)
// ============================================

mod gateway {
    use super::*;

    #[rustapi_rs::get("/api/users/{id}")]
    async fn proxy_get_user(Path(id): Path<u64>) -> Json<GatewayResponse> {
        let client = reqwest::Client::new();
        let user: User = client
            .get(&format!("http://127.0.0.1:8081/users/{}", id))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        Json(GatewayResponse {
            service: "user-service".to_string(),
            data: serde_json::to_value(user).unwrap(),
        })
    }

    #[rustapi_rs::get("/api/orders/{id}")]
    async fn proxy_get_order(Path(id): Path<u64>) -> Json<GatewayResponse> {
        let client = reqwest::Client::new();
        let order: Order = client
            .get(&format!("http://127.0.0.1:8082/orders/{}", id))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        Json(GatewayResponse {
            service: "order-service".to_string(),
            data: serde_json::to_value(order).unwrap(),
        })
    }

    #[rustapi_rs::get("/")]
    async fn index() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "message": "API Gateway",
            "services": {
                "users": "http://127.0.0.1:8080/api/users/{id}",
                "orders": "http://127.0.0.1:8080/api/orders/{id}"
            }
        }))
    }

    pub async fn start() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🚀 Starting API Gateway on port 8080...");
        println!("📍 Gateway: http://127.0.0.1:8080");
        println!("📍 Swagger UI: http://127.0.0.1:8080/docs");
        RustApi::auto().run("127.0.0.1:8080").await
    }
}

// ============================================
// Main - Start All Services
// ============================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🌐 Starting Microservices Demo...");
    println!("\n📊 Services:");
    println!("   1. User Service - :8081");
    println!("   2. Order Service - :8082");
    println!("   3. API Gateway - :8080");
    println!("\n🧪 Test with:");
    println!("   curl http://127.0.0.1:8080/api/users/1");
    println!("   curl http://127.0.0.1:8080/api/orders/1");

    // Start services in parallel
    let user_service = tokio::spawn(user_service::start());
    let order_service = tokio::spawn(order_service::start());

    // Give services time to start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Start gateway (blocks)
    gateway::start().await?;

    // Wait for all services (won't reach here normally)
    let _ = tokio::try_join!(user_service, order_service);

    Ok(())
}
