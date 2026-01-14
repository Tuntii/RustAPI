# rustapi-view: The Artist

**Lens**: "The Artist"
**Philosophy**: "Server-side rendering with modern tools."

## Tera Integration

We use **Tera**, a Jinja2-like template engine, for rendering HTML on the server.

```rust
async fn home(
    State(templates): State<Templates>
) -> View {
    let mut ctx = Context::new();
    ctx.insert("user", "Alice");
    
    View::new("home.html", ctx)
}
```

## Layouts and Inheritance

Tera supports template inheritance, allowing you to define a base layout (`base.html`) and extend it in child templates (`index.html`), keeping your frontend DRY.
