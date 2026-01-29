//! # Hello World Example
//!
//! The minimal RustAPI application demonstrating core concepts.
//!
//! ## Demonstrates
//! - `RustApi::auto()` for automatic route discovery
//! - Path parameter extraction with `Path<T>`
//! - JSON response serialization
//! - OpenAPI schema generation with `utoipa::ToSchema`
//!
//! ## Run
//! ```bash
//! cargo run -p hello-world
//! ```
//!
//! ## Test
//! ```bash
//! curl http://127.0.0.1:8080/hello/World
//! ```
//!
//! ## Cookbook
//! <https://tuntii.github.io/RustAPI/>

use rustapi_macros as rustapi;
use rustapi_rs::prelude::*;
use serde::Serialize;

#[derive(Serialize, rustapi::ToSchema)]
struct Message {
    greeting: String,
}

#[rustapi::get("/hello/{name}")]
async fn hello(Path(name): Path<String>) -> Json<Message> {
    Json(Message {
        greeting: format!("Hello, {name}!"),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::auto().run("0.0.0.0:8080").await
}
