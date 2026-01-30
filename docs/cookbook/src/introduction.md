# RustAPI Cookbook

Welcome to the **RustAPI Architecture Cookbook**. This documentation is designed to be the single source of truth for the project's philosophy, patterns, and practical implementation details.

> [!NOTE]
> This is a living document. As our architecture evolves, so will this cookbook.

> [!TIP]
> **v0.1.203**: RustAPI now uses a **native OpenAPI 3.1 generator**, removing the `utoipa` dependency. See the [Migration Guide](recipes/openapi_migration.md) for details.

## What is this?
This is not just API documentation. This is a collection of:
- **Keynotes**: High-level architectural decisions and "why" we made them.
- **Patterns**: The repeated structures (like `Action` and `Service`) that form the backbone of our code.
- **Recipes**: Practical, step-by-step guides for adding features, testing, and maintaining cleanliness.
- **Learning Paths**: Structured progressions with real-world examples.

## 🚀 New: Examples Repository

Looking for hands-on learning? Check out our **[Examples Repository](https://github.com/Tuntii/rustapi-rs-examples)** with 18 complete projects:

| Category | Examples |
|----------|----------|
| **Getting Started** | hello-world, crud-api |
| **Authentication** | auth-api (JWT), rate-limit-demo |
| **Database** | sqlx-crud, event-sourcing |
| **AI/LLM** | toon-api, mcp-server |
| **Real-time** | websocket, graphql-api |
| **Production** | microservices, serverless-lambda |

👉 See [Learning & Examples](learning/README.md) for structured learning paths.

## Visual Identity
This cookbook is styled with the **RustAPI Premium Dark** theme, focusing on readability, contrast, and modern "glassmorphism" aesthetics.

## Quick Start
- Want to add a feature? Jump to [Adding a New Feature](recipes/new_feature.md).
- Want to understand performance? Read [Performance Philosophy](architecture/performance.md).
- Need to check code quality? See [Maintenance](recipes/maintenance.md).
- **New to RustAPI?** Follow our [Learning Paths](learning/README.md).
- **Upgrading from v0.1.202?** Read the [OpenAPI Migration Guide](recipes/openapi_migration.md).

