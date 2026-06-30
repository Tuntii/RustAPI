//! RustAPI Cloud HTTP helpers (auth refresh, shared client).

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::config::{load_config, save_config, CloudConfig};

#[derive(Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    expires_in: u64,
}

#[derive(Deserialize)]
struct RefreshError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

pub fn cloud_http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")
}

/// Load config and return a valid access token, refreshing when the API returns 401.
pub async fn with_access_token<F, Fut, T>(mut operation: F) -> Result<T>
where
    F: FnMut(Client, String, String) -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut config = load_config()?;
    let cloud_url = config
        .cloud_url
        .clone()
        .unwrap_or_else(|| "https://api.rustapi.cloud".into());
    let token = config
        .token
        .clone()
        .ok_or_else(|| anyhow!("Not logged in. Run `cargo rustapi login` first."))?;

    let client = cloud_http_client()?;
    match operation(client.clone(), cloud_url.clone(), token.clone()).await {
        Ok(value) => Ok(value),
        Err(err) if looks_like_auth_failure(&err) => {
            refresh_tokens(&client, &cloud_url, &mut config).await?;
            let new_token = config.token.clone().expect("token after refresh");
            operation(client, cloud_url, new_token).await
        }
        Err(err) => Err(err),
    }
}

fn looks_like_auth_failure(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("401")
        || msg.contains("unauthorized")
        || msg.contains("invalid or expired token")
        || msg.contains("invalid/expired token")
}

pub async fn refresh_tokens(
    client: &Client,
    cloud_url: &str,
    config: &mut CloudConfig,
) -> Result<()> {
    let refresh = config
        .refresh_token
        .clone()
        .ok_or_else(|| anyhow!("Session expired. Run `cargo rustapi login` again."))?;

    let resp = client
        .post(format!("{}/auth/refresh", cloud_url.trim_end_matches('/')))
        .json(&serde_json::json!({ "refresh_token": refresh }))
        .send()
        .await
        .context("Failed to connect for token refresh")?;

    if !resp.status().is_success() {
        let body: RefreshError = resp.json().await.unwrap_or(RefreshError {
            error: "invalid_grant".into(),
            error_description: Some("Refresh failed".into()),
        });
        return Err(anyhow!(
            "{}: {}",
            body.error,
            body.error_description.unwrap_or_default()
        ));
    }

    let body: RefreshResponse = resp.json().await.context("Invalid refresh response")?;
    config.token = Some(body.access_token);
    config.refresh_token = Some(body.refresh_token);
    save_config(config)?;
    Ok(())
}
