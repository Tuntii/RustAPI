# RustAPI Examples

This directory contains comprehensive examples demonstrating RustAPI's features and use cases.

## 🌟 Getting Started Examples

### [hello-world](hello-world/)
**Difficulty**: ⭐ Beginner  
**Lines**: ~20  
The minimal RustAPI application. Perfect first example.

```bash
cargo run -p hello-world
```

### [crud-api](crud-api/)
**Difficulty**: ⭐⭐ Intermediate  
**Lines**: ~335  
Complete CRUD API with validation, error handling, and middleware.

```bash
cargo run -p crud-api
```

### [proof-of-concept](proof-of-concept/)
**Difficulty**: ⭐⭐ Intermediate  
**Lines**: ~200  
Showcase of various RustAPI features in one place.

```bash
cargo run -p proof-of-concept
```

---

## 🔐 Authentication & Security Examples

### [auth-api](auth-api/)
**Difficulty**: ⭐⭐⭐ Advanced  
**Lines**: ~450  
JWT authentication with login, registration, and protected routes.

```bash
cargo run -p auth-api
```

**Features**:
- JWT token generation & validation
- Password hashing with bcrypt
- Protected routes with `AuthUser<T>` extractor
- Token refresh mechanism

### [rate-limit-demo](rate-limit-demo/)
**Difficulty**: ⭐⭐ Intermediate  
**Lines**: ~120  
IP-based rate limiting with different configurations per endpoint.

```bash
cargo run -p rate-limit-demo
```

**Features**:
- Per-endpoint rate limits
- Burst support
- Rate limit headers
- 429 Too Many Requests handling

### [middleware-chain](middleware-chain/)
**Difficulty**: ⭐⭐⭐ Advanced  
**Lines**: ~180  
Custom middleware composition and execution order.

```bash
cargo run -p middleware-chain
```

**Features**:
- Request ID tracking
- Request timing
- Custom authentication
- Middleware composition

---

## 🗄️ Database Examples

### [sqlx-crud](sqlx-crud/)
**Difficulty**: ⭐⭐⭐ Advanced  
**Lines**: ~500+  
Full CRUD API with PostgreSQL integration using SQLx.

```bash
# Start PostgreSQL
docker run -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres

# Run example
cargo run -p sqlx-crud
```

**Features**:
- PostgreSQL with SQLx
- Database migrations
- Connection pooling
- Transaction management
- Async database queries

---

## 🤖 AI & LLM Examples

### [toon-api](toon-api/)
**Difficulty**: ⭐⭐ Intermediate  
**Lines**: ~200  
TOON format for token-optimized LLM responses.

```bash
cargo run -p toon-api
```

**Features**:
- TOON format serialization
- Content negotiation (JSON/TOON)
- Token count headers
- 50-58% token savings

### [mcp-server](mcp-server/)
**Difficulty**: ⭐⭐⭐ Advanced  
**Lines**: ~300  
Model Context Protocol server implementation.

```bash
cargo run -p mcp-server
```

**Features**:
- MCP protocol support
- Tool definitions
- Resource management
- AI agent integration

---

## 🌐 Real-time & Web Examples

### [websocket](websocket/)
**Difficulty**: ⭐⭐⭐ Advanced  
**Lines**: ~250  
Real-time WebSocket chat with broadcast channels.

```bash
cargo run -p websocket
```

**Features**:
- WebSocket connections
- Broadcast channels
- Pub/sub patterns
- Connection management

### [templates](templates/)
**Difficulty**: ⭐⭐ Intermediate  
**Lines**: ~200  
Server-side HTML rendering with Tera templates.

```bash
cargo run -p templates
```

**Features**:
- Tera template engine
- Template inheritance
- Type-safe context
- Static file serving

---

## 🏗️ Advanced Architecture Examples

### [graphql-api](graphql-api/)
**Difficulty**: ⭐⭐⭐⭐ Expert  
**Lines**: ~280  
GraphQL API with async-graphql integration.

```bash
cargo run -p graphql-api
```

**Features**:
- GraphQL queries & mutations
- Type-safe resolvers
- GraphQL Playground
- Schema introspection

### [microservices](microservices/)
**Difficulty**: ⭐⭐⭐⭐ Expert  
**Lines**: ~220  
Multi-service architecture with API Gateway pattern.

```bash
cargo run -p microservices
```

**Features**:
- API Gateway
- Service-to-service communication
- Multiple services in one binary
- Request routing & proxying

---

## 📊 Example Matrix

| Example | REST | WebSocket | Database | Auth | Templates | LLM | GraphQL |
|---------|------|-----------|----------|------|-----------|-----|---------|
| hello-world | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| crud-api | ✅ | ❌ | Memory | ❌ | ❌ | ❌ | ❌ |
| auth-api | ✅ | ❌ | Memory | ✅ | ❌ | ❌ | ❌ |
| sqlx-crud | ✅ | ❌ | PostgreSQL | ❌ | ❌ | ❌ | ❌ |
| toon-api | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| websocket | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| templates | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |
| mcp-server | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| rate-limit-demo | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| graphql-api | ✅ | ❌ | Memory | ❌ | ❌ | ❌ | ✅ |
| microservices | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| middleware-chain | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |

---

## 🚀 Running All Examples

```bash
# List all examples
cargo run --example

# Run specific example
cargo run -p <example-name>

# Run with logs
RUST_LOG=debug cargo run -p <example-name>

# Build all examples
cargo build --examples --release
```

---

## 📝 Creating Your Own Example

1. **Create directory**: `examples/my-example/`
2. **Add Cargo.toml**:
   ```toml
   [package]
   name = "my-example"
   version = "0.1.0"
   edition = "2021"

   [dependencies]
   rustapi-rs = { path = "../../crates/rustapi-rs" }
   tokio = { version = "1", features = ["full"] }
   serde = { version = "1", features = ["derive"] }
   ```

3. **Create src/main.rs**:
   ```rust
   use rustapi_rs::prelude::*;

   #[rustapi_rs::get("/")]
   async fn index() -> &'static str {
       "Hello from my example!"
   }

   #[tokio::main]
   async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
       RustApi::auto().run("127.0.0.1:8080").await
   }
   ```

4. **Add to workspace** in root `Cargo.toml`:
   ```toml
   members = [
       # ...
       "examples/my-example",
   ]
   ```

5. **Run it**:
   ```bash
   cargo run -p my-example
   ```

---

## 💡 Tips for Learning

1. **Start simple** — Begin with `hello-world`, then `crud-api`
2. **Read the code** — Examples are heavily commented
3. **Experiment** — Modify examples to understand behavior
4. **Check docs** — Visit http://127.0.0.1:8080/docs when running
5. **Ask questions** — Open a [Discussion](https://github.com/Tuntii/RustAPI/discussions)

---

## 🎯 Example Roadmap

### Coming Soon:
- [ ] **redis-cache** — Redis caching layer
- [ ] **sse-events** — Server-Sent Events
- [ ] **grpc-integration** — gRPC + REST hybrid
- [ ] **database-pooling** — Advanced connection pool management
- [ ] **distributed-tracing** — OpenTelemetry integration
- [ ] **kubernetes-ready** — Health checks, metrics, graceful shutdown

Want to contribute an example? See [CONTRIBUTING.md](../CONTRIBUTING.md)!
