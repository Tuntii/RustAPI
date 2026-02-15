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
        &copy; 2026 RustAPI
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

In your `main.rs`, initialize the `Templates` engine and inject it into the application state. Handlers can then extract it using `State<Templates>`.

```rust,no_run
use rustapi_rs::prelude::*;
use rustapi_view::{View, Templates};
use serde::Serialize;

#[derive(Serialize)]
struct User {
    name: String,
    is_admin: bool,
}

#[derive(Serialize)]
struct HomeContext {
    app_name: String,
    user: User,
    items: Vec<String>,
}

#[rustapi_rs::get("/")]
async fn index(templates: State<Templates>) -> View<HomeContext> {
    let context = HomeContext {
        app_name: "My Awesome App".to_string(),
        user: User {
            name: "Alice".to_string(),
            is_admin: true,
        },
        items: vec!["Apple".to_string(), "Banana".to_string(), "Cherry".to_string()],
    };

    // Render the "index.html" template with the context
    View::render(&templates, "index.html", context).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Initialize Template Engine
    // Loads all .html files from the "templates" directory
    let templates = Templates::new("templates/**/*.html")?;

    // 2. Add to State
    let app = RustApi::new()
        .state(templates)
        .route("/", get(index));

    println!("Listening on http://localhost:3000");
    app.run("0.0.0.0:3000").await.unwrap();

    Ok(())
}
```

## Template Reloading

In **Debug** mode (`cargo run`), `rustapi-view` automatically reloads templates from disk on every request. This means you can edit your `.html` files and refresh the browser to see changes instantly without recompiling.

In **Release** mode (`cargo run --release`), templates are compiled and cached for maximum performance.

## Asset Serving

To serve CSS, JS, and images, use `serve_static` on the `RustApi` builder.

```rust,ignore
let app = RustApi::new()
    .state(templates)
    .route("/", get(index))
    .serve_static("/assets", "assets"); // Serves files from ./assets at /assets
```
