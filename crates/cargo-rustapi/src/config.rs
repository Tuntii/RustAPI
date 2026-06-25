use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub login: String,
    pub tier: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CloudConfig {
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub user: Option<UserInfo>,
    #[serde(default)]
    pub last_login: Option<String>,
    #[serde(default)]
    pub cloud_url: Option<String>,
}

fn config_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());

    PathBuf::from(home).join(".rustapi")
}

fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var("RUSTAPI_CONFIG_PATH") {
        return PathBuf::from(path);
    }
    config_dir().join("config.json")
}

pub fn load_config() -> Result<CloudConfig> {
    let path = config_path();
    if !path.exists() {
        return Err(anyhow::anyhow!("Not logged in. Run `rustapi login` first."));
    }

    let json = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;

    serde_json::from_str(&json).context("Failed to parse config file")
}

pub fn save_config(config: &CloudConfig) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("Failed to create {}", dir.display()))?;

    let path = config_path();
    let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;

    Ok(())
}

pub fn clear_config() -> Result<()> {
    let path = config_path();
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove {}", path.display()))?;
    }
    Ok(())
}
