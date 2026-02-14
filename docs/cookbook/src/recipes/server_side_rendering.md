# Server-Side Rendering (SSR)

While RustAPI excels at building JSON APIs, it also supports server-side rendering using the `rustapi-view` crate, which leverages the [Tera](https://keats.github.io/tera/) template engine (inspired by Jinja2).

## Dependencies

Add the following to your `Cargo.toml`:

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["view"] }
serde = { version = "1.0", features = ["derive"] }
```

## Creating Templates

Create a `templates` directory in your project root.

**`templates/base.html`** (The layout):
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>{% block title %}My App{% endblock %}</title>
    <link rel="stylesheet" href="/assets/style.css">
</head>
<body>
    <nav>
        <a href="/">Home</a>
        <a href="/about">About</a>
    </nav>

    <main>
        {% block content %}{% endblock %}
    </main>

    <footer>
        &copy; 2025 RustAPI
    </footer>
</body>
</html>
```

**`templates/index.html`** (The page):
```html
{% extends "base.html" %}

{% block title %}Home - {{ app_name }}{% endblock %}

{% block content %}
    <h1>Welcome, {{ user.name }}!</h1>

    {% if user.is_admin %}
        <p>You have admin privileges.</p>
    {% endif %}

    <h2>Latest Items</h2>
    <ul>
    {% for item in items %}
        <li>{{ item }}</li>
    {% endfor %}
    </ul>
{% endblock %}
```

## Handling Requests

In your `main.rs`, use the `View` type and `Context`.

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_view::{View, Context};
use serde::Serialize;

#[derive(Serialize)]
struct User {
    name: String,
    is_admin: bool,
}

#[rustapi_rs::get("/")]
async fn index() -> View {
    // 1. Create context
    let mut ctx = Context::new();

    // 2. Insert data
    ctx.insert("app_name", "My Awesome App");

    let user = User {
        name: "Alice".to_string(),
        is_admin: true,
    };
    ctx.insert("user", &user);

    ctx.insert("items", &vec!["Apple", "Banana", "Cherry"]);

    // 3. Render template
    // RustAPI automatically loads templates from the "templates" directory
    View::new("index.html", ctx)
}

#[tokio::main]
async fn main() {
    // No special setup needed for View, it's auto-configured if the crate is present
    // and the "templates" directory exists.
    let app = RustApi::new().route("/", get(index));

    println!("Listening on http://localhost:3000");
    app.run("0.0.0.0:3000").await.unwrap();
}
```

## Template Reloading

In **Debug** mode (`cargo run`), `rustapi-view` automatically reloads templates from disk on every request. This means you can edit your `.html` files and refresh the browser to see changes instantly without recompiling.

In **Release** mode (`cargo run --release`), templates are compiled and cached for maximum performance.

## Asset Serving

To serve CSS, JS, and images, use `ServeDir` from `tower-http` (re-exported or available via `rustapi-extras` if configured, or just standard tower).

```rust,ignore
use tower_http::services::ServeDir;

let app = RustApi::new()
    .route("/", get(index))
    .nest_service("/assets", ServeDir::new("assets"));
```
