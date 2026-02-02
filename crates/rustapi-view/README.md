# rustapi-view

**Lens**: "The Artist"  
**Philosophy**: "Server-side rendering with modern tools."

Server-side rendering for RustAPI using Tera (Jinja2-like template engine).

## Features

- **Type-Safe Context**: Pass Rust structs directly to templates
- **Auto-Reload**: Templates reload automatically in debug mode—no restart required
- **Includes & Inheritance**: Master pages, blocks, and macros supported

## Tera Integration

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

## Full Example

**`src/main.rs`**
```rust
use rustapi_view::{View, Context};

#[rustapi_rs::get("/")]
async fn index() -> View {
    let mut ctx = Context::new();
    ctx.insert("title", "My Blog");
    ctx.insert("posts", &vec!["Post 1", "Post 2"]);
    
    View::new("index.html", ctx)
}
```

**`templates/index.html`**
```html
<h1>{{ title }}</h1>
<ul>
{% for post in posts %}
    <li>{{ post }}</li>
{% endfor %}
</ul>
```
