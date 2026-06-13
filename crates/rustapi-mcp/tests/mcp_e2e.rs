//! End-to-end integration tests for Native MCP.
//!
//! These tests spin up a real RustAPI HTTP server + MCP sidecar on ephemeral ports,
//! exercise the JSON-RPC transport (initialize, tools/list, tools/call), and verify
//! that tool invocations are proxied through the normal pipeline (including tag-based
//! exposure control).

use std::time::Duration;

use rustapi_rs::prelude::*;
use rustapi_rs::protocol::mcp::{McpConfig, McpServer, run_rustapi_and_mcp_with_shutdown};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

/// Simple response for a tagged tool.
#[derive(Serialize, Schema)]
struct Weather {
    city: String,
    temperature: i32,
    unit: &'static str,
}

/// Request body for a mutating tool.
#[derive(Serialize, Schema, Deserialize)]
struct ComputeRequest {
    a: i32,
    b: i32,
}

#[derive(Serialize, Schema)]
struct ComputeResponse {
    sum: i32,
}

// ------------------ Tagged tools (will be exposed) ------------------

#[rustapi_rs::get("/weather/{city}")]
#[rustapi_rs::tag("agent")]
#[rustapi_rs::summary("Get current weather for a city")]
async fn get_weather(Path(city): Path<String>) -> Json<Weather> {
    Json(Weather {
        city,
        temperature: 22,
        unit: "C",
    })
}

#[rustapi_rs::post("/compute")]
#[rustapi_rs::tag("agent")]
#[rustapi_rs::summary("Compute the sum of two numbers")]
async fn compute(Json(req): Json<ComputeRequest>) -> Json<ComputeResponse> {
    Json(ComputeResponse { sum: req.a + req.b })
}

// ------------------ Untagged / internal (must NOT be exposed) ------------------

#[rustapi_rs::get("/admin/secret")]
async fn admin_secret() -> &'static str {
    "top-secret"
}

#[tokio::test]
async fn test_mcp_initialize_and_filtered_tools_list() {
    let app = RustApi::auto();

    let mcp = McpServer::from_rustapi(
        &app,
        McpConfig::new()
            .name("test-mcp-server")
            .version("1.0.0-test")
            .allowed_tags(["agent"]),
    );

    // Ephemeral ports for both servers
    let http_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let http_addr = http_listener.local_addr().unwrap();
    drop(http_listener);

    let mcp_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mcp_addr = mcp_listener.local_addr().unwrap();
    drop(mcp_listener);

    let http_addr_str = format!("127.0.0.1:{}", http_addr.port());
    let mcp_addr_str = format!("127.0.0.1:{}", mcp_addr.port());

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let server_handle = tokio::spawn(async move {
        run_rustapi_and_mcp_with_shutdown(
            app,
            &http_addr_str,
            mcp,
            &mcp_addr_str,
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
    });

    // Give servers time to bind and start
    tokio::time::sleep(Duration::from_millis(250)).await;

    let client = reqwest::Client::new();
    let mcp_url = format!("http://127.0.0.1:{}/", mcp_addr.port());

    // --- initialize ---
    let init_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });

    let res = client
        .post(&mcp_url)
        .json(&init_body)
        .send()
        .await
        .expect("initialize request failed");
    assert_eq!(res.status(), 200);

    let init_resp: serde_json::Value = res.json().await.unwrap();
    assert_eq!(init_resp["jsonrpc"], "2.0");
    assert_eq!(init_resp["id"], 1);
    assert!(init_resp.get("result").is_some());
    let result = &init_resp["result"];
    assert_eq!(result["serverInfo"]["name"], "test-mcp-server");
    assert!(result["capabilities"]["tools"].is_object());

    // --- tools/list (only "agent" tagged) ---
    let list_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let res = client
        .post(&mcp_url)
        .json(&list_body)
        .send()
        .await
        .expect("tools/list request failed");
    assert_eq!(res.status(), 200);

    let list_resp: serde_json::Value = res.json().await.unwrap();
    let tools = &list_resp["result"]["tools"];
    let tool_names: Vec<String> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    // Should contain the two agent-tagged tools, but not the admin one
    assert!(tool_names.iter().any(|n| n.contains("get_weather") || n.contains("weather")));
    assert!(tool_names.iter().any(|n| n.contains("compute")));
    assert!(!tool_names.iter().any(|n| n.contains("secret") || n.contains("admin")));

    // Trigger shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(3), server_handle).await;
}

#[tokio::test]
async fn test_mcp_tool_call_get_with_path_param_and_post_body() {
    let app = RustApi::auto();

    let mcp = McpServer::from_rustapi(
        &app,
        McpConfig::new()
            .name("e2e-mcp")
            .version("0.0.0")
            .allowed_tags(["agent"]),
    );

    let http_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let http_addr = http_listener.local_addr().unwrap();
    drop(http_listener);

    let mcp_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mcp_addr = mcp_listener.local_addr().unwrap();
    drop(mcp_listener);

    let http_addr_str = format!("127.0.0.1:{}", http_addr.port());
    let mcp_addr_str = format!("127.0.0.1:{}", mcp_addr.port());

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let server_handle = tokio::spawn(async move {
        run_rustapi_and_mcp_with_shutdown(
            app,
            &http_addr_str,
            mcp,
            &mcp_addr_str,
            async move { let _ = shutdown_rx.await; },
        )
        .await
    });

    tokio::time::sleep(Duration::from_millis(250)).await;

    let client = reqwest::Client::new();
    let mcp_url = format!("http://127.0.0.1:{}/", mcp_addr.port());

    // First discover the exact tool names (generation depends on operation_id / slug rules)
    let list_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "list-for-call",
        "method": "tools/list"
    });
    let list_res = client
        .post(&mcp_url)
        .json(&list_body)
        .send()
        .await
        .expect("tools/list before calls failed");
    let list_body: serde_json::Value = list_res.json().await.unwrap();
    let tools = list_body["result"]["tools"].as_array().unwrap();

    let weather_tool = tools
        .iter()
        .find(|t| t["name"].as_str().unwrap_or("").contains("weather"))
        .expect("weather tool should be discoverable")
        ["name"]
        .as_str()
        .unwrap()
        .to_string();

    let compute_tool = tools
        .iter()
        .find(|t| t["name"].as_str().unwrap_or("").contains("compute"))
        .expect("compute tool should be discoverable")
        ["name"]
        .as_str()
        .unwrap()
        .to_string();

    // --- Call GET tool with path argument (using discovered name) ---
    let call_get = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "call-1",
        "method": "tools/call",
        "params": {
            "name": weather_tool,
            "arguments": { "city": "Istanbul" }
        }
    });

    let res = client
        .post(&mcp_url)
        .json(&call_get)
        .send()
        .await
        .expect("tools/call (GET) failed");
    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.unwrap();
    let result = &body["result"];
    assert_eq!(result["isError"], false, "GET tool call should succeed: {:?}", result);
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Istanbul") || text.contains("22"), "response text should contain city or temp: {}", text);

    // --- Call POST tool with body (using discovered name) ---
    let call_post = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "call-2",
        "method": "tools/call",
        "params": {
            "name": compute_tool,
            "arguments": { "a": 40, "b": 2 }
        }
    });

    let res = client
        .post(&mcp_url)
        .json(&call_post)
        .send()
        .await
        .expect("tools/call (POST) failed");
    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.unwrap();
    let result = &body["result"];
    assert_eq!(result["isError"], false, "POST tool call should succeed: {:?}", result);
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("42") || text.contains("sum"), "response should contain sum result: {}", text);

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(3), server_handle).await;
}

#[tokio::test]
async fn test_mcp_tool_not_found_for_untagged_route() {
    let app = RustApi::auto();

    let mcp = McpServer::from_rustapi(
        &app,
        McpConfig::new().allowed_tags(["agent"]), // only agent tools
    );

    let http_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let http_addr = http_listener.local_addr().unwrap();
    drop(http_listener);

    let mcp_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mcp_addr = mcp_listener.local_addr().unwrap();
    drop(mcp_listener);

    let http_addr_str = format!("127.0.0.1:{}", http_addr.port());
    let mcp_addr_str = format!("127.0.0.1:{}", mcp_addr.port());

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let server_handle = tokio::spawn(async move {
        run_rustapi_and_mcp_with_shutdown(
            app,
            &http_addr_str,
            mcp,
            &mcp_addr_str,
            async move { let _ = shutdown_rx.await; },
        )
        .await
    });

    tokio::time::sleep(Duration::from_millis(250)).await;

    let client = reqwest::Client::new();
    let mcp_url = format!("http://127.0.0.1:{}/", mcp_addr.port());

    // Try to call the untagged admin tool by a plausible generated name
    let bad_call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "tools/call",
        "params": {
            "name": "get_admin_secret",
            "arguments": {}
        }
    });

    let res = client.post(&mcp_url).json(&bad_call).send().await.unwrap();
    let body: serde_json::Value = res.json().await.unwrap();

    // Either we get a JSON-RPC error or a result with isError (our current impl uses result+isError for execution errors)
    // The ToolNotFound path currently surfaces as a successful response with isError + error text.
    if let Some(err) = body.get("error") {
        // protocol level error is also acceptable
        assert!(err["message"].as_str().unwrap_or("").contains("not found"));
    } else {
        let result = &body["result"];
        assert_eq!(result["isError"], true);
        let text = result["content"][0]["text"].as_str().unwrap_or("");
        assert!(text.to_lowercase().contains("not found") || text.contains("ToolNotFound"));
    }

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(3), server_handle).await;
}
