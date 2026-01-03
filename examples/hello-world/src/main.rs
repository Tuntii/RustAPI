use rustapi_rs::prelude::*;
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
struct Message {
    greeting: String,
}

#[rustapi_rs::get("/hello/{name}")]
async fn hello(Path(name): Path<String>) -> Json<Message> {
    Json(Message { greeting: format!("Hello, {name}!") })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    RustApi::auto().run("0.0.0.0:8080").await
}