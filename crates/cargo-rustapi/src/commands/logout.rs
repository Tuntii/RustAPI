use clap::Args;

use crate::config::{clear_config, load_config};

#[derive(Args, Debug, Clone)]
pub struct LogoutArgs;

pub async fn logout(_args: LogoutArgs) -> anyhow::Result<()> {
    let config = load_config()?;

    let login = config
        .user
        .as_ref()
        .map(|u| u.login.as_str())
        .unwrap_or("unknown");

    clear_config()?;

    println!("  \x1b[32m✓ Logged out\x1b[0m (was {})", login);
    Ok(())
}
