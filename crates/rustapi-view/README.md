# rustapi-view

Template rendering support for RustAPI framework using Tera templates.

## Features

- **Tera Templates**: Full Tera template engine support
- **Type-Safe Context**: Build template context from Rust structs
- **Auto-Reload**: Development mode auto-reloads templates (optional)
- **Response Types**: `View<T>` and `Html` response types
- **Layout Support**: Template inheritance and blocks

## Quick Start

```rust
use rustapi_rs::prelude::*;
use rustapi_view::{View, Templates};
use serde::Serialize;

#[derive(Serialize)]
struct HomeContext {
    title: String,
    user: Option<String>,
}

async fn home() -> View<HomeContext> {
    View::new("home.html", HomeContext {
        title: "Welcome".to_string(),
        user: Some("Alice".to_string()),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize templates from directory
    let templates = Templates::new("templates/**/*.html")?;

    RustApi::new()
        .state(templates)
        .route("/", get(home))
        .run("127.0.0.1:8080")
        .await
}
```

## Template Files

Create your templates in a `templates/` directory:

```html
<!-- templates/base.html -->
<!DOCTYPE html>
<html>
<head>
    <title>{% block title %}{{ title }}{% endblock %}</title>
</head>
<body>
    {% block content %}{% endblock %}
</body>
</html>

<!-- templates/home.html -->
{% extends "base.html" %}

{% block content %}
<h1>Welcome{% if user %}, {{ user }}{% endif %}!</h1>
{% endblock %}
```

## Context Building

```rust
use rustapi_view::{Context, View};

// From struct (requires Serialize)
let view = View::new("template.html", MyStruct { ... });

// From context builder
let view = View::with_context("template.html", |ctx| {
    ctx.insert("name", "Alice");
    ctx.insert("items", &vec!["a", "b", "c"]);
});
```

## Configuration

```rust
use rustapi_view::{Templates, TemplatesConfig};

// With configuration
let templates = Templates::with_config(TemplatesConfig {
    glob: "templates/**/*.html".to_string(),
    auto_reload: cfg!(debug_assertions), // Auto-reload in debug mode
    strict_mode: true, // Fail on undefined variables
});
```

## License

MIT OR Apache-2.0
