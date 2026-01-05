//! Template Rendering Example
//!
//! This example demonstrates Tera template support in RustAPI:
//! - Server-side HTML rendering
//! - Template inheritance (layouts)
//! - Context building
//! - Static file serving
//!
//! Run with: cargo run --package templates-example

use rustapi_rs::prelude::*;
use rustapi_rs::view::{ContextBuilder, Templates, View};

/// Contact form params
#[derive(Debug, Clone, Deserialize, IntoParams)]
struct ContactForm {
    name: Option<String>,
    message: Option<String>,
}

/// Home page context
#[derive(Serialize)]
struct HomeContext {
    title: String,
    features: Vec<Feature>,
}

#[derive(Serialize)]
struct Feature {
    name: String,
    description: String,
}

/// About page context
#[derive(Serialize)]
struct AboutContext {
    title: String,
    version: String,
    rust_version: String,
}

/// Contact form context
#[derive(Serialize)]
struct ContactContext {
    title: String,
    submitted: bool,
    name: Option<String>,
    message: Option<String>,
}

/// Blog post context
#[derive(Serialize)]
struct BlogContext {
    title: String,
    posts: Vec<BlogPost>,
}

#[derive(Serialize)]
struct BlogPost {
    id: u32,
    title: String,
    excerpt: String,
    author: String,
    date: String,
}

/// Home page handler
async fn home(State(templates): State<Templates>) -> View<HomeContext> {
    let features = vec![
        Feature {
            name: "Type-Safe".to_string(),
            description: "Compile-time route and schema validation".to_string(),
        },
        Feature {
            name: "Fast".to_string(),
            description: "Built on Tokio and Hyper for maximum performance".to_string(),
        },
        Feature {
            name: "Easy".to_string(),
            description: "Minimal boilerplate, intuitive API".to_string(),
        },
        Feature {
            name: "Documented".to_string(),
            description: "Auto-generated OpenAPI + Swagger UI".to_string(),
        },
    ];

    View::render(
        &templates,
        "index.html",
        HomeContext {
            title: "Home".to_string(),
            features,
        },
    )
    .await
}

/// About page handler
async fn about(State(templates): State<Templates>) -> View<AboutContext> {
    View::render(
        &templates,
        "about.html",
        AboutContext {
            title: "About".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            rust_version: "1.75+".to_string(),
        },
    )
    .await
}

/// Contact page handler (GET)
async fn contact_get(State(templates): State<Templates>) -> View<ContactContext> {
    View::render(
        &templates,
        "contact.html",
        ContactContext {
            title: "Contact".to_string(),
            submitted: false,
            name: None,
            message: None,
        },
    )
    .await
}

/// Contact form submission (POST)
async fn contact_post(
    State(templates): State<Templates>,
    Query(params): Query<ContactForm>,
) -> View<ContactContext> {
    tracing::info!("Contact form submitted: {:?}", params);

    View::render(
        &templates,
        "contact.html",
        ContactContext {
            title: "Contact".to_string(),
            submitted: true,
            name: params.name,
            message: params.message,
        },
    )
    .await
}

/// Blog listing page
async fn blog(State(templates): State<Templates>) -> View<BlogContext> {
    let posts = vec![
        BlogPost {
            id: 1,
            title: "Getting Started with RustAPI".to_string(),
            excerpt: "Learn how to build your first API with RustAPI...".to_string(),
            author: "RustAPI Team".to_string(),
            date: "2026-01-05".to_string(),
        },
        BlogPost {
            id: 2,
            title: "WebSocket Support in RustAPI".to_string(),
            excerpt: "Real-time communication made easy...".to_string(),
            author: "RustAPI Team".to_string(),
            date: "2026-01-04".to_string(),
        },
        BlogPost {
            id: 3,
            title: "Template Rendering with Tera".to_string(),
            excerpt: "Server-side rendering for your web apps...".to_string(),
            author: "RustAPI Team".to_string(),
            date: "2026-01-03".to_string(),
        },
    ];

    View::render(
        &templates,
        "blog.html",
        BlogContext {
            title: "Blog".to_string(),
            posts,
        },
    )
    .await
}

/// Dynamic context example using ContextBuilder
async fn dynamic(State(templates): State<Templates>) -> View<()> {
    let context = ContextBuilder::new()
        .insert("title", &"Dynamic Page")
        .insert("items", &vec!["One", "Two", "Three"])
        .insert("count", &3)
        .insert_if("show_banner", &true, |_| true)
        .build();

    View::render_context(&templates, "dynamic.html", &context).await
}

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("templates_example=debug".parse().unwrap())
                .add_directive("info".parse().unwrap()),
        )
        .init();

    // Initialize templates from the templates directory
    let templates = Templates::new("examples/templates/templates/**/*.html")?;

    let addr = "127.0.0.1:8080";
    tracing::info!("🚀 Server running at http://{}", addr);

    RustApi::new()
        .state(templates)
        .route("/", get(home))
        .route("/about", get(about))
        .route("/contact", get(contact_get))
        .route("/contact", post(contact_post))
        .route("/blog", get(blog))
        .route("/dynamic", get(dynamic))
        .serve_static("/static", "examples/templates/static")
        .run(addr)
        .await
}
