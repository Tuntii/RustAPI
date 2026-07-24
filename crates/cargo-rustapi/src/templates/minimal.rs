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
rustapi-rs = {{ version = "{version}"{features} }}
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
tracing-subscriber = "0.3"
"#,
        name = name,
        version = common::rustapi_rs_version(),
        features = common::features_to_cargo(features),
    );
    fs::create_dir_all(format!("{name}/src")).await?;

    let wants_mcp = features.iter().any(|f| f == "protocol-mcp");

    // main.rs
    let main_rs = if wants_mcp {
        r#"use rustapi_rs::prelude::*;
use rustapi_rs::protocol::mcp::{
    run_rustapi_and_mcp_with_shutdown, McpConfig, McpServer, ToolPolicy,
};
use serde::Serialize;
use tokio::signal;

#[derive(Serialize, Schema)]
struct Hello {
    message: String,
}

/// Tagged routes are exposed as MCP tools for agents.
#[rustapi_rs::get("/")]
#[rustapi_rs::tag("public")]
#[rustapi_rs::summary("Hello world")]
async fn hello() -> Json<Hello> {
    Json(Hello {
        message: "Hello, World!".to_string(),
    })
}

#[rustapi_rs::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let mcp_port = std::env::var("MCP_PORT").unwrap_or_else(|_| "9090".to_string());
    let addr = format!("127.0.0.1:{}", port);
    let mcp_addr = format!("127.0.0.1:{}", mcp_port);

    let app = RustApi::auto().docs("/docs");

    let mut mcp_cfg = McpConfig::new()
        .name("hello-mcp")
        .allowed_tags(["public"])
        .tool_policy(ToolPolicy::ReadOnly);
    if let Ok(token) = std::env::var("RUSTAPI_MCP_TOKEN") {
        if !token.is_empty() {
            mcp_cfg = mcp_cfg.admin_token(token);
        }
    }
    let mcp = McpServer::from_rustapi(&app, mcp_cfg);

    println!("🚀 Server running at http://{}", addr);
    println!("📚 API docs at http://{}/docs", addr);
    println!("🧠 MCP tools at http://{}", mcp_addr);

    run_rustapi_and_mcp_with_shutdown(app, &addr, mcp, &mcp_addr, async {
        signal::ctrl_c().await.ok();
    })
    .await
}
"#
        .to_string()
    } else {
        r#"use rustapi_rs::prelude::*;
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
"#
        .to_string()
    };

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
