# Learning & Examples

Welcome to the RustAPI learning resources! This section provides structured learning paths and links to comprehensive real-world examples to help you master the framework.

## 📚 Learning Resources

### Official Examples Repository

We maintain a comprehensive examples repository with **18 real-world projects** demonstrating RustAPI's full capabilities:

🔗 **[rustapi-rs-examples](https://github.com/Tuntii/rustapi-rs-examples)** - Complete examples from hello-world to production microservices

### Cookbook Internal Path

If you prefer reading through documentation first, follow this path through the cookbook:

1. **Foundations**: Start with [Handlers & Extractors](../concepts/handlers.md) and [System Overview](../architecture/system_overview.md).
2. **Core Crates**: Read about [rustapi-core](../crates/rustapi_core.md) and [rustapi-macros](../crates/rustapi_macros.md).
3. **Building Blocks**: Try the [Creating Resources](../recipes/crud_resource.md) recipe.
4. **Security**: Implement [JWT Authentication](../recipes/jwt_auth.md) and [CSRF Protection](../recipes/csrf_protection.md).
5. **Advanced**: Explore [Performance Tuning](../recipes/high_performance.md) and [HTTP/3](../recipes/http3_quic.md).
6. **Background Jobs**: Master [rustapi-jobs](../crates/rustapi_jobs.md) for async processing.

### Why Use the Examples Repository?

| Benefit | Description |
|---------|-------------|
| **Structured Learning** | Progress from beginner → intermediate → advanced |
| **Real-world Patterns** | Production-ready implementations you can adapt |
| **Feature Discovery** | Find examples by the features you want to learn |
| **AI-Friendly** | Module-level docs help AI assistants understand your code |

---

## 🎯 Learning Paths

Choose a learning path based on your goals:

### 🚀 Path 1: REST API Developer

Build production-ready REST APIs with RustAPI.

| Step | Example | Skills Learned |
|------|---------|----------------|
| 1 | `hello-world` | Basic routing, handlers, server setup |
| 2 | `crud-api` | CRUD operations, extractors, error handling |
| 3 | `auth-api` | JWT authentication, protected routes |
| 4 | `middleware-chain` | Custom middleware, logging, CORS |
| 5 | `sqlx-crud` | Database integration, async queries |

**Related Cookbook Recipes:**
- [Creating Resources](../recipes/crud_resource.md)
- [JWT Authentication](../recipes/jwt_auth.md)
- [Database Integration](../recipes/db_integration.md)

---

### 🏗️ Path 2: Microservices Architect

Design and build distributed systems with RustAPI.

| Step | Example | Skills Learned |
|------|---------|----------------|
| 1 | `crud-api` | Service fundamentals |
| 2 | `middleware-chain` | Cross-cutting concerns |
| 3 | `rate-limit-demo` | API protection, throttling |
| 4 | `microservices` | Service communication patterns |
| 5 | `microservices-advanced` | Service discovery, Consul integration |
| 6 | Background jobs (conceptual) | Background processing with `rustapi-jobs`, Redis/Postgres backends |

> Note: The **Background jobs (conceptual)** step refers to using the `rustapi-jobs` crate rather than a standalone example project.
**Related Cookbook Recipes:**
- [rustapi-jobs](../crates/rustapi_jobs.md)
- [Custom Middleware](../recipes/custom_middleware.md)
- [Production Tuning](../recipes/high_performance.md)
- [Deployment](../recipes/deployment.md)

---

### ⚡ Path 3: Real-time Applications

Build interactive, real-time features with WebSockets.

| Step | Example | Skills Learned |
|------|---------|----------------|
| 1 | `hello-world` | Framework basics |
| 2 | `websocket` | WebSocket connections, message handling |
| 3 | `middleware-chain` | Connection middleware |
| 4 | `graphql-api` | Subscriptions, real-time queries |

**Related Cookbook Recipes:**
- [Real-time Chat](../recipes/websockets.md)
- [Handlers & Extractors](../concepts/handlers.md)

---

### 🤖 Path 4: AI/LLM Integration

Build AI-friendly APIs with TOON format and MCP support.

| Step | Example | Skills Learned |
|------|---------|----------------|
| 1 | `crud-api` | API fundamentals |
| 2 | `toon-api` | TOON format for LLM-friendly responses |
| 3 | `mcp-server` | Model Context Protocol implementation |
| 4 | `proof-of-concept` | Combining multiple AI features |

**Related Cookbook Recipes:**
- [rustapi-toon: The Diplomat](../crates/rustapi_toon.md)

---

### 🏢 Path 5: Enterprise Platform

Build robust, observable, and secure systems.

| Step | Feature | Description |
|------|---------|-------------|
| 1 | **Observability** | Set up [OpenTelemetry and Structured Logging](../crates/rustapi_extras.md#observability) |
| 2 | **Resilience** | Implement [Circuit Breakers and Retries](../recipes/resilience.md) |
| 3 | **Advanced Security** | Add [OAuth2 and Security Headers](../crates/rustapi_extras.md#advanced-security) |
| 4 | **Optimization** | Configure [Caching and Deduplication](../crates/rustapi_extras.md#optimization) |
| 5 | **Background Jobs** | Implement [Reliable Job Queues](../crates/rustapi_jobs.md) |

**Related Cookbook Recipes:**
- [rustapi-extras: The Toolbox](../crates/rustapi_extras.md)
- [rustapi-jobs: The Workhorse](../crates/rustapi_jobs.md)
- [Resilience Patterns](../recipes/resilience.md)

---

## 📦 Examples by Category

### Getting Started
| Example | Description | Difficulty |
|---------|-------------|------------|
| `hello-world` | Minimal RustAPI server | ⭐ Beginner |
| `crud-api` | Complete CRUD operations | ⭐ Beginner |

### Authentication & Security
| Example | Description | Difficulty |
|---------|-------------|------------|
| `auth-api` | JWT authentication flow | ⭐⭐ Intermediate |
| `middleware-chain` | Middleware composition | ⭐⭐ Intermediate |
| `rate-limit-demo` | API rate limiting | ⭐⭐ Intermediate |

### Database Integration
| Example | Description | Difficulty |
|---------|-------------|------------|
| `sqlx-crud` | SQLx with PostgreSQL/SQLite | ⭐⭐ Intermediate |
| `event-sourcing` | Event sourcing patterns | ⭐⭐⭐ Advanced |

### AI & LLM
| Example | Description | Difficulty |
|---------|-------------|------------|
| `toon-api` | TOON format responses | ⭐⭐ Intermediate |
| `mcp-server` | Model Context Protocol | ⭐⭐⭐ Advanced |

### Real-time & GraphQL
| Example | Description | Difficulty |
|---------|-------------|------------|
| `websocket` | WebSocket chat example | ⭐⭐ Intermediate |
| `graphql-api` | GraphQL with async-graphql | ⭐⭐⭐ Advanced |

### Production Patterns
| Example | Description | Difficulty |
|---------|-------------|------------|
| `microservices` | Basic service communication | ⭐⭐⭐ Advanced |
| `microservices-advanced` | Consul service discovery | ⭐⭐⭐ Advanced |
| `serverless-lambda` | AWS Lambda deployment | ⭐⭐⭐ Advanced |

---

## 🔧 Feature Matrix

Find examples by the RustAPI features they demonstrate:

| Feature | Examples |
|---------|----------|
| `#[get]`, `#[post]` macros | All examples |
| `State<T>` extractor | `crud-api`, `auth-api`, `sqlx-crud` |
| `Json<T>` extractor | `crud-api`, `auth-api`, `graphql-api` |
| `ValidatedJson<T>` | `auth-api`, `crud-api` |
| JWT (`jwt` feature) | `auth-api`, `microservices` |
| CORS (`cors` feature) | `middleware-chain`, `auth-api` |
| Rate Limiting | `rate-limit-demo`, `auth-api` |
| WebSockets (`ws` feature) | `websocket`, `graphql-api` |
| TOON (`toon` feature) | `toon-api`, `mcp-server` |
| OAuth2 (`oauth2-client`) | `auth-api` (extended) |
| Circuit Breaker | `microservices` |
| OpenTelemetry (`otel`) | `microservices-advanced` |
| OpenAPI/Swagger | All examples |

---

## 🚦 Getting Started with Examples

### Clone the Repository

```bash
git clone https://github.com/Tuntii/rustapi-rs-examples.git
cd rustapi-rs-examples
```

### Run an Example

```bash
cd hello-world
cargo run
```

### Test an Example

```bash
# Most examples have tests
cargo test

# Or use the TestClient
cd ../crud-api
cargo test
```

### Explore the Structure

Each example includes:
- `README.md` - Detailed documentation with API endpoints
- `src/main.rs` - Entry point with server setup
- `src/handlers.rs` - Request handlers (where applicable)
- `Cargo.toml` - Dependencies and feature flags
- Tests demonstrating the TestClient

---

## 📖 Cross-Reference: Cookbook ↔ Examples

| Cookbook Recipe | Related Examples |
|-----------------|------------------|
| [Creating Resources](../recipes/crud_resource.md) | `crud-api`, `sqlx-crud` |
| [JWT Authentication](../recipes/jwt_auth.md) | `auth-api` |
| [CSRF Protection](../recipes/csrf_protection.md) | `auth-api`, `middleware-chain` |
| [Database Integration](../recipes/db_integration.md) | `sqlx-crud`, `event-sourcing` |
| [File Uploads](../recipes/file_uploads.md) | `file-upload` (coming soon) |
| [Custom Middleware](../recipes/custom_middleware.md) | `middleware-chain` |
| [Real-time Chat](../recipes/websockets.md) | `websocket` |
| [Production Tuning](../recipes/high_performance.md) | `microservices-advanced` |
| [Resilience Patterns](../recipes/resilience.md) | `microservices` |
| [Deployment](../recipes/deployment.md) | `serverless-lambda` |

---

## 💡 Contributing Examples

Have a great example to share? We welcome contributions!

1. Fork the [rustapi-rs-examples](https://github.com/Tuntii/rustapi-rs-examples) repository
2. Create your example following our structure guidelines
3. Add comprehensive documentation in README.md
4. Submit a pull request

### Example Guidelines

- Include a clear README with prerequisites and API endpoints
- Add code comments explaining RustAPI-specific patterns
- Include working tests using `rustapi-testing`
- List the feature flags used

---

## 🔗 Additional Resources

- **[RustAPI GitHub](https://github.com/Tuntii/RustAPI)** - Framework source code
- **[API Reference](https://docs.rs/rustapi-rs)** - Generated documentation
- **[Feature Flags Reference](../reference/)** - All available features
- **[Architecture Guide](../architecture/system_overview.md)** - How RustAPI works internally

---

> 💬 **Need help?** Open an issue in the examples repository or join our community discussions!
