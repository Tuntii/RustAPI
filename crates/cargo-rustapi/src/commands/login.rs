use anyhow::{anyhow, Context};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_CLOUD_URL: &str = "https://api.rustapi.cloud";
const POLL_INTERVAL_SECS: u64 = 3;
const POLL_TIMEOUT_SECS: u64 = 900; // 15 minutes

#[derive(Args, Debug, Clone)]
pub struct LoginArgs {
    /// RustAPI Cloud API URL
    #[arg(long, default_value = DEFAULT_CLOUD_URL)]
    pub cloud_url: String,

    /// Do not open browser automatically
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

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    token: Option<String>,
    refresh_token: Option<String>,
    user: Option<UserInfo>,
    last_login: Option<String>,
    cloud_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct UserInfo {
    login: String,
    tier: String,
    avatar_url: Option<String>,
}

pub async fn login(args: LoginArgs) -> anyhow::Result<()> {
    let client = Client::new();
    let cloud_url = args.cloud_url.trim_end_matches('/');

    // Step 1: Request device code
    let device_resp: DeviceResponse = client
        .post(format!("{}/auth/device", cloud_url))
        .send()
        .await
        .context("Failed to connect to RustAPI Cloud")?
        .json()
        .await
        .context("Invalid response from /auth/device")?;

    // Step 2: Show activation instructions
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

    // Step 3: Open browser
    if !args.no_browser {
        let _ = open::that(&device_resp.verification_uri_complete);
    }

    // Step 4: Poll for token
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
                "Device code expired. Please run `rustapi login` again."
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

                // Step 5: Get user info
                let user_info: UserInfo = match client
                    .post(format!("{}/auth/whoami", cloud_url))
                    .json(&serde_json::json!({ "grant_type": "", "device_code": access_token }))
                    .send()
                    .await
                {
                    Ok(resp) => match resp.json::<serde_json::Value>().await {
                        Ok(v) if v.get("sub").is_some() => UserInfo {
                            login: v["login"].as_str().unwrap_or("unknown").into(),
                            tier: v["tier"].as_str().unwrap_or("hobby").into(),
                            avatar_url: v["avatar_url"].as_str().map(String::from),
                        },
                        _ => continue,
                    },
                    Err(_) => continue,
                };

                // Step 6: Save config
                let config = ConfigFile {
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

fn config_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());

    PathBuf::from(home).join(".rustapi").join("config.json")
}

fn save_config(config: &ConfigFile) -> anyhow::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create ~/.rustapi directory")?;
    }
    let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    std::fs::write(&path, json).context("Failed to write config file")?;
    Ok(())
}
