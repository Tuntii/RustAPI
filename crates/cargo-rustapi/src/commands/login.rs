use anyhow::{anyhow, Context};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::{save_config, CloudConfig, UserInfo};

const DEFAULT_CLOUD_URL: &str = "https://api.rustapi.cloud";
const POLL_INTERVAL_SECS: u64 = 3;
const POLL_TIMEOUT_SECS: u64 = 900;

#[derive(Args, Debug, Clone)]
pub struct LoginArgs {
    #[arg(long, default_value = DEFAULT_CLOUD_URL)]
    pub cloud_url: String,

    #[arg(long)]
    pub no_browser: bool,
}

#[derive(Deserialize)]
struct DeviceResponse {
    device_code: String,
    user_code: String,
    #[allow(dead_code)]
    verification_uri: String,
    verification_uri_complete: String,
    #[allow(dead_code)]
    expires_in: u32,
    interval: u32,
}

#[derive(Serialize)]
struct TokenRequest {
    grant_type: String,
    device_code: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TokenResponse {
    Success {
        access_token: String,
        refresh_token: String,
        #[allow(dead_code)]
        token_type: String,
        #[allow(dead_code)]
        expires_in: u32,
    },
    Error {
        error: String,
        #[serde(default)]
        error_description: Option<String>,
    },
}

pub async fn login(args: LoginArgs) -> anyhow::Result<()> {
    let client = Client::new();
    let cloud_url = args.cloud_url.trim_end_matches('/');

    let device_resp: DeviceResponse = client
        .post(format!("{}/auth/device", cloud_url))
        .send()
        .await
        .context("Failed to connect to RustAPI Cloud")?
        .json()
        .await
        .context("Invalid response from /auth/device")?;

    println!();
    println!("  \x1b[1mRustAPI Cloud Login\x1b[0m");
    println!("  ─────────────────────");
    println!();
    println!("  Open this URL in your browser:");
    println!();
    println!("  \x1b[36m{}\x1b[0m", device_resp.verification_uri_complete);
    println!();
    println!(
        "  And enter this code: \x1b[1;33m{}\x1b[0m",
        device_resp.user_code
    );
    println!();

    if !args.no_browser {
        let _ = open::that(&device_resp.verification_uri_complete);
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("  {spinner} Waiting for authorization...").unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

    let start = std::time::Instant::now();
    let interval = Duration::from_secs(std::cmp::max(
        device_resp.interval as u64,
        POLL_INTERVAL_SECS,
    ));

    loop {
        tokio::time::sleep(interval).await;

        if start.elapsed().as_secs() > POLL_TIMEOUT_SECS {
            spinner.finish_with_message("Login timed out");
            return Err(anyhow!(
                "Device code expired. Please run `cargo rustapi login` again."
            ));
        }

        let token_resp: TokenResponse = match client
            .post(format!("{}/auth/token", cloud_url))
            .json(&TokenRequest {
                grant_type: "urn:ietf:params:oauth:grant-type:device_code".into(),
                device_code: device_resp.device_code.clone(),
            })
            .send()
            .await
        {
            Ok(resp) => match resp.json().await {
                Ok(body) => body,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        match token_resp {
            TokenResponse::Success {
                access_token,
                refresh_token,
                ..
            } => {
                spinner.finish_and_clear();

                // Get user info
                let user_info = fetch_user_info(&client, cloud_url, &access_token).await?;

                let config = CloudConfig {
                    token: Some(access_token),
                    refresh_token: Some(refresh_token),
                    user: Some(user_info),
                    last_login: Some(chrono::Utc::now().to_rfc3339()),
                    cloud_url: Some(cloud_url.to_string()),
                };

                save_config(&config)?;

                println!(
                    "  \x1b[32m✓ Logged in as {}\x1b[0m",
                    config.user.as_ref().unwrap().login
                );
                println!("  Tier: {}", config.user.as_ref().unwrap().tier);
                println!();
                return Ok(());
            }
            TokenResponse::Error {
                error,
                error_description,
            } => {
                if error == "authorization_pending" {
                    continue;
                }
                let desc = error_description.unwrap_or_default();
                spinner.finish_with_message(format!("Error: {} - {}", error, desc));
                return Err(anyhow!("{}: {}", error, desc));
            }
        }
    }
}

async fn fetch_user_info(
    client: &Client,
    cloud_url: &str,
    token: &str,
) -> anyhow::Result<UserInfo> {
    let resp = client
        .post(format!("{}/auth/whoami", cloud_url))
        .json(&serde_json::json!({
            "grant_type": "",
            "device_code": token,
        }))
        .send()
        .await
        .context("Failed to fetch user info")?;

    let v: serde_json::Value = resp.json().await.context("Invalid whoami response")?;

    Ok(UserInfo {
        login: v["login"].as_str().unwrap_or("unknown").into(),
        tier: v["tier"].as_str().unwrap_or("hobby").into(),
        avatar_url: v["avatar_url"].as_str().map(String::from),
    })
}
