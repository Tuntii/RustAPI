//! End-to-end integration tests for Native MCP.
//!
//! These tests spin up a real RustAPI HTTP server + MCP sidecar on ephemeral ports,
//! exercise the JSON-RPC transport (initialize, tools/list, tools/call), and verify
//! that tool invocations are proxied through the normal pipeline (including tag-based
//! exposure control).

use std::time::Duration;

use rustapi_rs::prelude::*;
use rustapi_rs::protocol::mcp::{
    run_rustapi_and_mcp_with_shutdown, InvocationMode, McpConfig, McpServer,
};
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
            .allowed_tags(["agent"])
            .tool_policy(rustapi_mcp::ToolPolicy::All), // test needs write tools
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
        run_rustapi_and_mcp_with_shutdown(app, &http_addr_str, mcp, &mcp_addr_str, async move {
            let _ = shutdown_rx.await;
        })
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
    assert!(tool_names
        .iter()
        .any(|n| n.contains("get_weather") || n.contains("weather")));
    assert!(tool_names.iter().any(|n| n.contains("compute")));
    assert!(!tool_names
        .iter()
        .any(|n| n.contains("secret") || n.contains("admin")));

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
        run_rustapi_and_mcp_with_shutdown(app, &http_addr_str, mcp, &mcp_addr_str, async move {
            let _ = shutdown_rx.await;
        })
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
        .expect("weather tool should be discoverable")["name"]
        .as_str()
        .unwrap()
        .to_string();

    let compute_tool = tools
        .iter()
        .find(|t| t["name"].as_str().unwrap_or("").contains("compute"))
        .expect("compute tool should be discoverable")["name"]
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
    assert_eq!(
        result["isError"], false,
        "GET tool call should succeed: {:?}",
        result
    );
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("Istanbul") || text.contains("22"),
        "response text should contain city or temp: {}",
        text
    );

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
    assert_eq!(
        result["isError"], false,
        "POST tool call should succeed: {:?}",
        result
    );
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("42") || text.contains("sum"),
        "response should contain sum result: {}",
        text
    );

    // Shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(3), server_handle).await;
}

/// Real benchmark: proxy (with live HTTP server) vs in-process for 1000 sequential tool calls.
///
/// This test is ignored by default because wall-clock timing assertions can be flaky
/// under CI load. Run explicitly with `cargo test -- --ignored`.
#[tokio::test]
#[ignore]
async fn bench_proxy_vs_inprocess() {
    use tokio::sync::oneshot;
    let n = 1000usize;

    // --- Discover tool name (same for both) ---
    let app_disc = RustApi::auto();
    let mcp_disc = McpServer::from_rustapi(&app_disc, McpConfig::new().allowed_tags(["agent"]).tool_policy(rustapi_mcp::ToolPolicy::All));
    let tools = mcp_disc.list_tools().await.unwrap();
    let tool_name = tools
        .iter()
        .find(|t| t.name.contains("weather") || t.name.contains("get_weather"))
        .map(|t| t.name.clone())
        .expect("expected a weather tool in the test module");

    let call_req = rustapi_rs::protocol::mcp::ToolCallRequest {
        name: tool_name,
        arguments: [("city".to_string(), serde_json::json!("Berlin"))].into(),
    };

    // === In-process (dispatcher, zero network) ===
    let app_in = RustApi::auto();
    let mcp_in = McpServer::from_rustapi(
        &app_in,
        McpConfig::new()
            .allowed_tags(["agent"])
            .tool_policy(rustapi_mcp::ToolPolicy::All)
            .invocation_mode(InvocationMode::InProcess),
    );

    // warm up
    let _ = mcp_in.call_tool(call_req.clone()).await;

    let start = std::time::Instant::now();
    for _ in 0..n {
        let _ = mcp_in.call_tool(call_req.clone()).await.unwrap();
    }
    let inproc_dur = start.elapsed();

    // === Proxy with live HTTP server ===
    let app_p = RustApi::auto();
    let http_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let http_addr = http_listener.local_addr().unwrap();
    drop(http_listener);
    let http_addr_str = format!("127.0.0.1:{}", http_addr.port());

    let mcp_p = McpServer::from_rustapi(
        &app_p,
        McpConfig::new()
            .allowed_tags(["agent"])
            .tool_policy(rustapi_mcp::ToolPolicy::All)
            .invocation_mode(InvocationMode::Proxy),
    );

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let http_addr_for_spawn = http_addr_str.clone();
    let server_handle = tokio::spawn(async move {
        app_p
            .run_with_shutdown(&http_addr_for_spawn, async move {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Set base manually (or runner would do it)
    let mcp_p = mcp_p.with_http_base(format!("http://{}", http_addr_str));

    // warm up proxy path
    let _ = mcp_p.call_tool(call_req.clone()).await;

    let start = std::time::Instant::now();
    for _ in 0..n {
        let _ = mcp_p.call_tool(call_req.clone()).await.unwrap();
    }
    let proxy_dur = start.elapsed();

    // shutdown
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(3), server_handle).await;

    println!(
        "\n=== Live Server Benchmark ({} sequential tool calls) ===",
        n
    );
    println!(
        "In-process (direct): {:>8.3?}  avg: {:>6.1?} per call",
        inproc_dur,
        inproc_dur / n as u32
    );
    println!(
        "Proxy (live HTTP)  : {:>8.3?}  avg: {:>6.1?} per call",
        proxy_dur,
        proxy_dur / n as u32
    );

    let speedup = proxy_dur.as_secs_f64() / inproc_dur.as_secs_f64();
    println!("Speedup: {:.1}x (in-process is faster)", speedup);

    // Sanity
    assert!(
        inproc_dur < proxy_dur,
        "in-process should be faster than proxy with live server"
    );
}

#[tokio::test]
async fn test_mcp_tool_not_found_for_untagged_route() {
    let app = RustApi::auto();

    let mcp = McpServer::from_rustapi(
        &app,
        McpConfig::new()
            .allowed_tags(["agent"])
            .tool_policy(rustapi_mcp::ToolPolicy::ReadOnly), // only agent tools
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
        run_rustapi_and_mcp_with_shutdown(app, &http_addr_str, mcp, &mcp_addr_str, async move {
            let _ = shutdown_rx.await;
        })
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
