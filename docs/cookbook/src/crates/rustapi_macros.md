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
