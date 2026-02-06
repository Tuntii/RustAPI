//! CLI commands for replay management.
//!
//! Communicates with a running RustAPI server's `/__rustapi/replays` admin endpoints
//! via HTTP. Does not import `rustapi-extras` directly.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use console::style;

/// Replay debugging commands.
///
/// Manage recorded HTTP request/response pairs for time-travel debugging.
#[derive(Subcommand, Debug)]
pub enum ReplayArgs {
    /// List recorded replay entries
    List(ReplayListArgs),

    /// Show a single replay entry
    Show(ReplayShowArgs),

    /// Replay a recorded request against a target URL
    Run(ReplayRunArgs),

    /// Replay and compute diff against original response
    Diff(ReplayDiffArgs),
}

/// Arguments for `replay list`
#[derive(Args, Debug)]
pub struct ReplayListArgs {
    /// Server URL
    #[arg(short, long, default_value = "http://localhost:8080")]
    pub server: String,

    /// Admin bearer token
    #[arg(short, long)]
    pub token: String,

    /// Maximum number of entries to return
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Filter by HTTP method
    #[arg(short, long)]
    pub method: Option<String>,

    /// Filter by path substring
    #[arg(short, long)]
    pub path: Option<String>,
}

/// Arguments for `replay show`
#[derive(Args, Debug)]
pub struct ReplayShowArgs {
    /// Replay entry ID
    pub id: String,

    /// Server URL
    #[arg(short, long, default_value = "http://localhost:8080")]
    pub server: String,

    /// Admin bearer token
    #[arg(short, long)]
    pub token: String,
}

/// Arguments for `replay run`
#[derive(Args, Debug)]
pub struct ReplayRunArgs {
    /// Replay entry ID
    pub id: String,

    /// Target URL to replay the request against
    #[arg(short = 'T', long)]
    pub target: String,

    /// Server URL
    #[arg(short, long, default_value = "http://localhost:8080")]
    pub server: String,

    /// Admin bearer token
    #[arg(short, long)]
    pub token: String,
}

/// Arguments for `replay diff`
#[derive(Args, Debug)]
pub struct ReplayDiffArgs {
    /// Replay entry ID
    pub id: String,

    /// Target URL to replay and diff against
    #[arg(short = 'T', long)]
    pub target: String,

    /// Server URL
    #[arg(short, long, default_value = "http://localhost:8080")]
    pub server: String,

    /// Admin bearer token
    #[arg(short, long)]
    pub token: String,
}

/// Execute a replay subcommand.
pub async fn replay(args: ReplayArgs) -> Result<()> {
    match args {
        ReplayArgs::List(a) => cmd_list(a).await,
        ReplayArgs::Show(a) => cmd_show(a).await,
        ReplayArgs::Run(a) => cmd_run(a).await,
        ReplayArgs::Diff(a) => cmd_diff(a).await,
    }
}

/// Build a reqwest client with the admin bearer token.
fn build_client(token: &str) -> Result<(reqwest::Client, reqwest::header::HeaderMap)> {
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    let auth_value = format!("Bearer {}", token);
    headers.insert(
        reqwest::header::AUTHORIZATION,
        auth_value
            .parse()
            .context("Invalid token format")?,
    );
    Ok((client, headers))
}

async fn cmd_list(args: ReplayListArgs) -> Result<()> {
    let (client, headers) = build_client(&args.token)?;

    let mut url = format!("{}/__rustapi/replays", args.server.trim_end_matches('/'));
    let mut params = Vec::new();
    if let Some(limit) = args.limit {
        params.push(format!("limit={}", limit));
    }
    if let Some(ref method) = args.method {
        params.push(format!("method={}", method));
    }
    if let Some(ref path) = args.path {
        params.push(format!("path={}", path));
    }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    let resp = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .context("Failed to connect to server")?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let msg = body["message"].as_str().unwrap_or("Unknown error");
        anyhow::bail!("Server returned {}: {}", status, msg);
    }

    let entries = body["entries"].as_array();
    let count = body["count"].as_u64().unwrap_or(0);
    let total = body["total"].as_u64().unwrap_or(0);

    println!(
        "{} Showing {} of {} replay entries\n",
        style("Replay Entries").bold().cyan(),
        count,
        total
    );

    if let Some(entries) = entries {
        if entries.is_empty() {
            println!("  No entries found.");
        } else {
            println!(
                "  {:<38} {:<7} {:<30} {:<6} {:<8}",
                style("ID").underlined(),
                style("Method").underlined(),
                style("Path").underlined(),
                style("Status").underlined(),
                style("Duration").underlined(),
            );

            for entry in entries {
                let id = entry["id"].as_str().unwrap_or("-");
                let method = entry["request"]["method"].as_str().unwrap_or("-");
                let path = entry["request"]["path"].as_str().unwrap_or("-");
                let status_code = entry["response"]["status"].as_u64().unwrap_or(0);
                let duration = entry["meta"]["duration_ms"].as_u64().unwrap_or(0);

                let status_styled = if status_code >= 500 {
                    style(status_code.to_string()).red()
                } else if status_code >= 400 {
                    style(status_code.to_string()).yellow()
                } else {
                    style(status_code.to_string()).green()
                };

                println!(
                    "  {:<38} {:<7} {:<30} {:<6} {:>5}ms",
                    style(id).dim(),
                    method,
                    path,
                    status_styled,
                    duration,
                );
            }
        }
    }

    println!();
    Ok(())
}

async fn cmd_show(args: ReplayShowArgs) -> Result<()> {
    let (client, headers) = build_client(&args.token)?;

    let url = format!(
        "{}/__rustapi/replays/{}",
        args.server.trim_end_matches('/'),
        args.id
    );

    let resp = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .context("Failed to connect to server")?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let msg = body["message"].as_str().unwrap_or("Unknown error");
        anyhow::bail!("Server returned {}: {}", status, msg);
    }

    println!("{}", style("Replay Entry").bold().cyan());
    println!();

    // Request section
    let req = &body["request"];
    println!("  {} {} {}", style("Request:").bold(), req["method"].as_str().unwrap_or("-"), req["uri"].as_str().unwrap_or("-"));
    if let Some(headers_obj) = req["headers"].as_object() {
        for (k, v) in headers_obj {
            println!("    {}: {}", style(k).dim(), v.as_str().unwrap_or("-"));
        }
    }
    if let Some(body_str) = req["body"].as_str() {
        println!("    {}", style("Body:").bold());
        print_json_indented(body_str, 6);
    }

    println!();

    // Response section
    let resp_data = &body["response"];
    let status_code = resp_data["status"].as_u64().unwrap_or(0);
    let status_styled = if status_code >= 500 {
        style(status_code.to_string()).red().bold()
    } else if status_code >= 400 {
        style(status_code.to_string()).yellow().bold()
    } else {
        style(status_code.to_string()).green().bold()
    };
    println!("  {} {}", style("Response:").bold(), status_styled);
    if let Some(headers_obj) = resp_data["headers"].as_object() {
        for (k, v) in headers_obj {
            println!("    {}: {}", style(k).dim(), v.as_str().unwrap_or("-"));
        }
    }
    if let Some(body_str) = resp_data["body"].as_str() {
        println!("    {}", style("Body:").bold());
        print_json_indented(body_str, 6);
    }

    println!();

    // Meta section
    let meta = &body["meta"];
    println!("  {}", style("Meta:").bold());
    println!("    Duration: {}ms", meta["duration_ms"].as_u64().unwrap_or(0));
    if let Some(ip) = meta["client_ip"].as_str() {
        println!("    Client IP: {}", ip);
    }
    if let Some(req_id) = meta["request_id"].as_str() {
        println!("    Request ID: {}", req_id);
    }

    println!();
    Ok(())
}

async fn cmd_run(args: ReplayRunArgs) -> Result<()> {
    let (client, headers) = build_client(&args.token)?;

    let url = format!(
        "{}/__rustapi/replays/{}/run?target={}",
        args.server.trim_end_matches('/'),
        args.id,
        args.target
    );

    println!(
        "{} Replaying {} against {}...",
        style("Replay").bold().cyan(),
        style(&args.id).dim(),
        style(&args.target).yellow()
    );

    let resp = client
        .post(&url)
        .headers(headers)
        .send()
        .await
        .context("Failed to connect to server")?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let msg = body["message"].as_str().unwrap_or("Unknown error");
        anyhow::bail!("Server returned {}: {}", status, msg);
    }

    println!();
    println!("  {} Original status: {}", style("Original:").bold(), body["original_response"]["status"]);
    println!("  {} Replayed status: {}", style("Replayed:").bold(), body["replayed_response"]["status"]);

    if let Some(body_str) = body["replayed_response"]["body"].as_str() {
        println!();
        println!("  {}", style("Replayed Body:").bold());
        print_json_indented(body_str, 4);
    }

    println!();
    Ok(())
}

async fn cmd_diff(args: ReplayDiffArgs) -> Result<()> {
    let (client, headers) = build_client(&args.token)?;

    let url = format!(
        "{}/__rustapi/replays/{}/diff?target={}",
        args.server.trim_end_matches('/'),
        args.id,
        args.target
    );

    println!(
        "{} Replaying {} against {} and computing diff...",
        style("Diff").bold().cyan(),
        style(&args.id).dim(),
        style(&args.target).yellow()
    );

    let resp = client
        .post(&url)
        .headers(headers)
        .send()
        .await
        .context("Failed to connect to server")?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.context("Failed to parse response")?;

    if !status.is_success() {
        let msg = body["message"].as_str().unwrap_or("Unknown error");
        anyhow::bail!("Server returned {}: {}", status, msg);
    }

    let diff = &body["diff"];
    let has_diff = diff["has_diff"].as_bool().unwrap_or(false);

    println!();

    if !has_diff {
        println!(
            "  {} No differences found!",
            style("MATCH").green().bold()
        );
    } else {
        println!(
            "  {} Differences detected:",
            style("DIFF").red().bold()
        );
        println!();

        // Status diff
        if let Some(status_diff) = diff["status_diff"].as_array() {
            if status_diff.len() == 2 {
                println!(
                    "    Status: {} -> {}",
                    style(status_diff[0].to_string()).red(),
                    style(status_diff[1].to_string()).green(),
                );
            }
        }

        // Header diffs
        if let Some(header_diffs) = diff["header_diffs"].as_array() {
            if !header_diffs.is_empty() {
                println!("    {}", style("Header differences:").bold());
                for hd in header_diffs {
                    let field = &hd["field"];
                    let original = hd["original"].as_str().unwrap_or("<missing>");
                    let replayed = hd["replayed"].as_str().unwrap_or("<missing>");
                    println!(
                        "      {}: {} -> {}",
                        style(format!("{}", field)).dim(),
                        style(original).red(),
                        style(replayed).green(),
                    );
                }
            }
        }

        // Body diff
        if let Some(body_diff) = diff["body_diff"].as_object() {
            if let Some(field_diffs) = body_diff["field_diffs"].as_array() {
                if !field_diffs.is_empty() {
                    println!("    {}", style("Body field differences:").bold());
                    for fd in field_diffs {
                        let field = &fd["field"];
                        let original = fd["original"].as_str().unwrap_or("<missing>");
                        let replayed = fd["replayed"].as_str().unwrap_or("<missing>");
                        println!(
                            "      {}: {} -> {}",
                            style(format!("{}", field)).dim(),
                            style(original).red(),
                            style(replayed).green(),
                        );
                    }
                }
            }
        }
    }

    println!();
    Ok(())
}

/// Pretty-print a JSON string with indentation.
fn print_json_indented(json_str: &str, indent: usize) {
    let prefix = " ".repeat(indent);
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Ok(pretty) = serde_json::to_string_pretty(&value) {
            for line in pretty.lines() {
                println!("{}{}", prefix, line);
            }
            return;
        }
    }
    // Fallback: print raw
    for line in json_str.lines() {
        println!("{}{}", prefix, line);
    }
}
