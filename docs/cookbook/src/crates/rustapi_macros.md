# rustapi-macros: The Magic

`rustapi-macros` reduces boilerplate by generating code at compile time.

## `#[debug_handler]`

The most important macro for beginners. Rust's error messages for complex generic traits (like `Handler`) can be notoriously difficult to understand.

If your handler doesn't implement the `Handler` trait (e.g., because you used an argument that isn't a valid Extractor), the compiler might give you an error spanning the entire `RustApi::new()` chain, miles away from the actual problem.

**`#[debug_handler]` fixes this.**

It verifies the handler function *in isolation* and produces clear error messages pointing exactly to the invalid argument.

```rust
#[debug_handler]
async fn handler(
    // Compile Error: "String" does not implement FromRequest. 
    // Did you mean "Json<String>" or "Body"?
    body: String 
) { ... }
```

## `#[derive(FromRequest)]`

Automatically implement `FromRequest` for your structs.

```rust
#[derive(FromRequest)]
struct MyExtractor {
    // These fields must themselves be Extractors
    header: HeaderMap,
    body: Json<MyData>,
}

// Now you can use it in a handler
async fn handler(input: MyExtractor) {
    println!("{:?}", input.header);
}
```

This is heavily used to group multiple extractors into a single struct (often called the "Parameter Object" pattern), keeping function signatures clean.

## Route Metadata Macros

RustAPI provides several attribute macros for enriching OpenAPI documentation:

### `#[rustapi::tag]`

Groups endpoints under a common tag in Swagger UI:

```rust
#[rustapi::get("/users")]
#[rustapi::tag("Users")]
async fn list_users() -> Json<Vec<User>> { ... }
```

### `#[rustapi::summary]` & `#[rustapi::description]`

Adds human-readable documentation:

```rust
#[rustapi::get("/users/{id}")]
#[rustapi::summary("Get user by ID")]
#[rustapi::description("Returns a single user by their unique identifier.")]
async fn get_user(Path(id): Path<i64>) -> Json<User> { ... }
```

### `#[rustapi::param]`

Customizes the OpenAPI schema type for path parameters. This is essential when the auto-inferred type is incorrect:

```rust
use uuid::Uuid;

// Without #[param], the `id` parameter would be documented as "integer"
// because of the naming convention. With #[param], it's correctly documented as UUID.
#[rustapi::get("/items/{id}")]
#[rustapi::param(id, schema = "uuid")]
async fn get_item(Path(id): Path<Uuid>) -> Json<Item> {
    find_item(id).await
}
```

**Supported schema types:** `"uuid"`, `"integer"`, `"int32"`, `"string"`, `"number"`, `"boolean"`

**Alternative syntax:**
```rust
#[rustapi::param(id = "uuid")]  // Shorter form
```

