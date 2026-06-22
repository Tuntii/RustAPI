use clap::Args;

use crate::config::load_config;

#[derive(Args, Debug, Clone)]
pub struct WhoamiArgs;

pub async fn whoami(_args: WhoamiArgs) -> anyhow::Result<()> {
    let config = load_config()?;

    match config.user {
        Some(user) => {
            println!("  Login:    {}", user.login);
            println!("  Tier:     {}", user.tier);
            if let Some(avatar) = &user.avatar_url {
                println!("  Avatar:   {}", avatar);
            }
            if let Some(ref url) = config.cloud_url {
                println!("  Cloud:    {}", url);
            }
            if let Some(ref last) = config.last_login {
                println!("  Last:     {}", last);
            }
            Ok(())
        }
        None => {
            println!("  Token is set but no user info available.");
            Ok(())
        }
    }
}
