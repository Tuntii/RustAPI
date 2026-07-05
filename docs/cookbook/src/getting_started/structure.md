# Project Structure

RustAPI projects follow a standard, modular structure designed for scalability.

```
my-api/
├── Cargo.toml          // Dependencies and workspace config
├── src/
│   ├── handlers/       // Request handlers (Controllers)
│   │   ├── mod.rs      
│   │   └── items.rs    // Example resource handler
│   ├── models/         // Data structures and Schema
│   │   ├── mod.rs      
│   ├── error.rs        // Custom error types
│   └── main.rs         // Application entry point & Router
└── .env.example        // Environment variables template
```

## Key Files

### `src/main.rs`
The heart of your application. This is where you configure the `RustApi` builder, register routes, and set up state.

### `src/handlers/`
Where your business logic lives. Handlers are async functions that take extractors (like `Json`, `Path`, `State`) and return responses.

### `src/models/`
Your data types. By deriving `Schema`, they automatically appear in your OpenAPI documentation.

### `src/error.rs`
Centralized error handling. Mapping your `AppError` to `ApiError` allows you to simply return `Result<T, AppError>` in your handlers.
