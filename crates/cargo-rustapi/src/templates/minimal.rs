//! Minimal project template

use super::common;
use anyhow::Result;
use tokio::fs;

pub async fn generate(name: &str, features: &[String]) -> Result<()> {
    // Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
rustapi-rs = {{ version = "0.1"{features} }}
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
tracing-subscriber = "0.3"
"#,
        name = name,
        features = common::features_to_cargo(features),
    );
    fs::create_dir_all(format!("{name}/src")).await?;

    // main.rs
    let main_rs = r#"use rustapi_rs::prelude::*;
use serde::Serialize;

#[derive(Serialize, Schema)]
struct Hello {
    message: String,
}

async fn hello() -> Json<Hello> {
    Json(Hello {
        message: "Hello, World!".to_string(),
    })
}

#[rustapi_rs::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);

    println!("🚀 Server running at http://{}", addr);

    RustApi::new()
        .route("/", get(hello))
        .docs("/docs")
        .run(&addr)
        .await
}
"#;

    // Write files in parallel for better performance
    let f1 = async {
        fs::write(format!("{name}/Cargo.toml"), cargo_toml)
            .await
            .map_err(anyhow::Error::from)
    };
    let f2 = async {
        fs::write(format!("{name}/src/main.rs"), main_rs)
            .await
            .map_err(anyhow::Error::from)
    };
    let f3 = common::generate_gitignore(name);

    tokio::try_join!(f1, f2, f3)?;

    Ok(())
}
