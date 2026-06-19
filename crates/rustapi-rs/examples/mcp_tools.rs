//! Demonstrates running a normal RustAPI HTTP server side-by-side with a
//! Native MCP server so that LLMs / agents (Claude, Cursor, etc.) can discover
//! and call your endpoints as tools.
//!
//! Uses `InvocationMode::InProcess` for zero-overhead tool calls (still
//! goes through full validation + middleware).
//!
//! Only routes that carry the "agent" tag are exposed via MCP.
//!
//! Run with:
//!   cargo run -p rustapi-rs --example mcp_tools --features protocol-mcp
//!
//! Then:
//! - Normal HTTP API is on http://127.0.0.1:8080
//! - MCP endpoint (for agents) is on http://127.0.0.1:9090
//!
//! Try from another terminal:
//!   curl -X POST http://127.0.0.1:9090 \
//!     -H 'content-type: application/json' \
//!     -d '{"jsonrpc":"2.0","id":1,"method":"initialize"}'
//!
//!   curl -X POST http://127.0.0.1:9090 \
//!     -H 'content-type: application/json' \
//!     -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
//!
//!   # Use the exact name returned by tools/list for the call
//!   curl -X POST http://127.0.0.1:9090 \
//!     -H 'content-type: application/json' \
//!     -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_agent_weather_city","arguments":{"city":"Istanbul"}}}'

use rustapi_rs::prelude::*;
use rustapi_rs::protocol::mcp::{
    run_rustapi_and_mcp_with_shutdown, InvocationMode, McpConfig, McpServer,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Schema)]
struct Weather {
    city: String,
    temperature: i32,
    unit: &'static str,
}

#[derive(Deserialize, Serialize, Schema)]
struct SumRequest {
    a: i32,
    b: i32,
}

#[derive(Serialize, Schema)]
struct SumResponse {
    sum: i32,
}

/// This route will be exposed as an MCP tool because of the "agent" tag.
#[rustapi_rs::get("/agent/weather/{city}")]
#[rustapi_rs::tag("agent")]
#[rustapi_rs::summary("Get weather information for a city")]
async fn get_weather(Path(city): Path<String>) -> Json<Weather> {
    Json(Weather {
        city,
        temperature: 23,
        unit: "C",
    })
}

/// Another exposed tool (POST with JSON body).
#[rustapi_rs::post("/agent/sum")]
#[rustapi_rs::tag("agent")]
#[rustapi_rs::summary("Add two integers and return the result")]
async fn sum(Json(req): Json<SumRequest>) -> Json<SumResponse> {
    Json(SumResponse { sum: req.a + req.b })
}

/// This route is deliberately NOT tagged — it will NOT appear in MCP discovery.
#[rustapi_rs::get("/admin/internal-config")]
async fn internal_only() -> &'static str {
    "this-should-never-be-visible-to-agents"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = RustApi::auto();

    // Only routes carrying at least one of the allowed tags become MCP tools.
    // This is the primary safety mechanism.
    let mcp = McpServer::from_rustapi(
        &app,
        McpConfig::new()
            .name("rustapi-mcp-demo")
            .version("0.1.0")
            .description("Demo RustAPI instance exposing selected endpoints to AI agents via MCP")
            .allowed_tags(["agent"])
            .invocation_mode(InvocationMode::InProcess),
    );

    let http_addr = "0.0.0.0:8080";
    let mcp_addr = "0.0.0.0:9090";

    println!("🚀 HTTP API listening on   http://{http_addr}");
    println!("🧠 MCP server (for agents) http://{mcp_addr}");
    println!();
    println!("Useful MCP JSON-RPC calls (use tools/list to discover exact tool names):");
    println!("  curl -X POST http://{mcp_addr} -H 'content-type: application/json' \\");
    println!("       -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}}'");
    println!();
    println!("  curl -X POST http://{mcp_addr} -H 'content-type: application/json' \\");
    println!("       -d '{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}}'");
    println!();
    println!("Press Ctrl+C to stop both servers cleanly.");

    run_rustapi_and_mcp_with_shutdown(app, http_addr, mcp, mcp_addr, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;

    Ok(())
}
