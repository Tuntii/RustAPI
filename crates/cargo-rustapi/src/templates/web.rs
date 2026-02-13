//! Web project template with Tera templates

use super::common;
use anyhow::Result;
use tokio::fs;

pub async fn generate(name: &str, features: &[String]) -> Result<()> {
    // Add the protocol-view feature for template rendering support
    let mut all_features = features.to_vec();
    if !all_features.contains(&"protocol-view".to_string()) {
        all_features.push("protocol-view".to_string());
    }

    // Cargo.toml - rustapi-view is accessed through rustapi-rs when protocol-view is enabled
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
rustapi-rs = {{ version = "0.1"{features} }}
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}
"#,
        name = name,
        features = common::features_to_cargo(&all_features),
    );
    fs::write(format!("{name}/Cargo.toml"), cargo_toml).await?;

    // Create directories
    fs::create_dir_all(format!("{name}/src/handlers")).await?;
    fs::create_dir_all(format!("{name}/templates")).await?;
    fs::create_dir_all(format!("{name}/static")).await?;

    // main.rs
    let main_rs = r#"mod handlers;

use rustapi_rs::prelude::*;
use rustapi_rs::protocol::view::Templates;

#[rustapi_rs::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    // Initialize templates
    let templates = Templates::new("templates/**/*.html")?;

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);

    tracing::info!("🚀 Server running at http://{}", addr);

    RustApi::new()
        .state(templates)
        // Pages
        .route("/", get(handlers::home))
        .route("/about", get(handlers::about))
        // Static files
        .serve_static("/static", "./static")
        .run(&addr)
        .await
}
"#;
    fs::write(format!("{name}/src/main.rs"), main_rs).await?;

    // handlers/mod.rs
    let handlers_mod = r#"//! Page handlers

use rustapi_rs::prelude::*;
use rustapi_rs::protocol::view::{Templates, View};
use serde::Serialize;

#[derive(Serialize)]
pub struct HomeContext {
    pub title: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct AboutContext {
    pub title: String,
    pub version: String,
}

/// Home page
pub async fn home(State(templates): State<Templates>) -> View<HomeContext> {
    View::render(&templates, "index.html", HomeContext {
        title: "Home".to_string(),
        message: "Welcome to RustAPI!".to_string(),
    }).await
}

/// About page
pub async fn about(State(templates): State<Templates>) -> View<AboutContext> {
    View::render(&templates, "about.html", AboutContext {
        title: "About".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }).await
}
"#;
    fs::write(format!("{name}/src/handlers/mod.rs"), handlers_mod).await?;

    // templates/base.html
    let base_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}{{ title }}{% endblock %} - RustAPI</title>
    <link rel="stylesheet" href="/static/style.css">
    {% block head %}{% endblock %}
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
        <p>Built with RustAPI</p>
    </footer>
    
    {% block scripts %}{% endblock %}
</body>
</html>
"#;
    fs::write(format!("{name}/templates/base.html"), base_html).await?;

    // templates/index.html
    let index_html = r#"{% extends "base.html" %}

{% block content %}
<h1>{{ message }}</h1>
<p>This is a RustAPI web application with Tera templates.</p>

<h2>Features</h2>
<ul>
    <li>Server-side rendering with Tera</li>
    <li>Static file serving</li>
    <li>Layout inheritance</li>
</ul>
{% endblock %}
"#;
    fs::write(format!("{name}/templates/index.html"), index_html).await?;

    // templates/about.html
    let about_html = r#"{% extends "base.html" %}

{% block content %}
<h1>About</h1>
<p>Version: {{ version }}</p>
<p>RustAPI is a FastAPI-like web framework for Rust.</p>
{% endblock %}
"#;
    fs::write(format!("{name}/templates/about.html"), about_html).await?;

    // static/style.css
    let style_css = r#"* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    line-height: 1.6;
    color: #333;
    max-width: 800px;
    margin: 0 auto;
    padding: 20px;
}

nav {
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid #eee;
}

nav a {
    margin-right: 1rem;
    color: #0066cc;
    text-decoration: none;
}

nav a:hover {
    text-decoration: underline;
}

main {
    min-height: calc(100vh - 200px);
}

h1 {
    margin-bottom: 1rem;
    color: #222;
}

h2 {
    margin-top: 2rem;
    margin-bottom: 0.5rem;
}

p {
    margin-bottom: 1rem;
}

ul {
    margin-left: 2rem;
    margin-bottom: 1rem;
}

footer {
    margin-top: 3rem;
    padding-top: 1rem;
    border-top: 1px solid #eee;
    color: #666;
    font-size: 0.9rem;
}
"#;
    fs::write(format!("{name}/static/style.css"), style_css).await?;

    // .gitignore and .env.example
    common::generate_gitignore(name).await?;
    common::generate_env_example(name).await?;

    Ok(())
}
