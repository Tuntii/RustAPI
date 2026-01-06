# Microservices Example

This example demonstrates a microservices architecture using RustAPI with an API Gateway pattern.

## Architecture

```
                    ┌─────────────────┐
                    │   API Gateway   │
                    │    (Port 8080)  │
                    └────────┬────────┘
                             │
                ┌────────────┴─────────────┐
                │                          │
        ┌───────▼────────┐        ┌───────▼────────┐
        │ User Service   │        │ Order Service  │
        │  (Port 8081)   │        │  (Port 8082)   │
        └────────────────┘        └────────────────┘
```

## Services

### 1. API Gateway (Port 8080)
- Routes requests to appropriate services
- Handles authentication & rate limiting
- Aggregates responses from multiple services
- Provides unified API to clients

### 2. User Service (Port 8081)
- Manages user data
- Handles user CRUD operations
- Internal service (not exposed to public)

### 3. Order Service (Port 8082)
- Manages order data
- Handles order processing
- Internal service (not exposed to public)

## Running

```bash
cargo run -p microservices
```

This starts all three services simultaneously:
- Gateway: http://127.0.0.1:8080
- User Service: http://127.0.0.1:8081
- Order Service: http://127.0.0.1:8082

## Testing

### Via API Gateway (recommended)
```bash
# Get user via gateway
curl http://127.0.0.1:8080/api/users/1

# Get order via gateway
curl http://127.0.0.1:8080/api/orders/1
```

### Direct service access (for testing)
```bash
# Direct user service
curl http://127.0.0.1:8081/users/1

# Direct order service
curl http://127.0.0.1:8082/orders/1
```

## Key Concepts

### Service-to-Service Communication
The gateway uses `reqwest` to make HTTP calls to backend services:

```rust
#[rustapi_rs::get("/api/users/{id}")]
async fn proxy_get_user(Path(id): Path<u64>) -> Json<GatewayResponse> {
    let client = reqwest::Client::new();
    let user: User = client
        .get(&format!("http://127.0.0.1:8081/users/{}", id))
        .send()
        .await?
        .json()
        .await?;
    
    Json(GatewayResponse {
        service: "user-service".to_string(),
        data: serde_json::to_value(user)?,
    })
}
```

### Service Registry (Future Enhancement)
For production, implement service discovery:
- **Consul** — Service registry & health checks
- **etcd** — Distributed configuration
- **Kubernetes** — Container orchestration

## Patterns Demonstrated

1. **API Gateway** — Single entry point for all clients
2. **Service Proxy** — Gateway forwards requests to services
3. **Response Aggregation** — Combine data from multiple services
4. **Service Isolation** — Each service has its own database/state

## Production Enhancements

### 1. Add Circuit Breaker
```rust
// Prevent cascading failures
if service_is_down {
    return fallback_response();
}
```

### 2. Implement Load Balancing
```rust
// Round-robin across service instances
let user_service_urls = vec![
    "http://user-service-1:8081",
    "http://user-service-2:8081",
];
let url = user_service_urls[request_count % 2];
```

### 3. Add Distributed Tracing
```rust
// Track requests across services
use opentelemetry::trace::Tracer;
let span = tracer.start("api-gateway");
```

### 4. Implement Service Mesh
- **Istio** — Traffic management, security, observability
- **Linkerd** — Lightweight service mesh for Kubernetes

## Benefits

✅ **Scalability** — Scale services independently  
✅ **Resilience** — Failure isolation between services  
✅ **Flexibility** — Use different tech stacks per service  
✅ **Team autonomy** — Teams own their services  

## Trade-offs

⚠️ **Complexity** — More moving parts to manage  
⚠️ **Network latency** — Inter-service calls add overhead  
⚠️ **Data consistency** — Distributed transactions are hard  
⚠️ **Debugging** — Harder to trace issues across services  

## When to Use

- **Large teams** — Multiple teams working on different features
- **Different scaling needs** — Some services need more resources
- **Technology diversity** — Need different languages/frameworks
- **Independent deployments** — Deploy services separately
