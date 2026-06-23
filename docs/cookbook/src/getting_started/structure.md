# Project Structure

RustAPI projects follow a standard, modular structure designed for scalability.

```
my-api/
â”œâ”€â”€ Cargo.toml          // Dependencies and workspace config
â”œâ”€â”€ src/
â”‚   â”œâ”€â”€ handlers/       // Request handlers (Controllers)
â”‚   â”‚   â”œâ”€â”€ mod.rs      
â”‚   â”‚   â””â”€â”€ items.rs    // Example resource handler
â”‚   â”œâ”€â”€ models/         // Data structures and Schema
â”‚   â”‚   â”œâ”€â”€ mod.rs      
â”‚   â”œâ”€â”€ error.rs        // Custom error types
â”‚   â””â”€â”€ main.rs         // Application entry point & Router
â””â”€â”€ .env.example        // Environment variables template
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
