use anyhow::Context;
use serde::Deserialize;

use crate::cloud;

#[derive(Deserialize)]
struct DeployListItem {
    id: String,
    project_name: String,
    status: String,
    url: Option<String>,
    created_at: String,
}

pub async fn deploys_list() -> anyhow::Result<()> {
    cloud::with_access_token(|client, cloud_url, token| async move {
        let response = client
            .get(format!("{}/deploys", cloud_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to fetch deploys")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Deploy list failed ({}): {}", status, body));
        }

        let items: Vec<DeployListItem> = response
            .json()
            .await
            .context("Invalid deploy list response")?;

        if items.is_empty() {
            println!("No deploys yet. Run `cargo rustapi deploy cloud` from a project.");
            return Ok(());
        }

        println!("{:<36} {:<18} {:<10} URL", "PROJECT", "DEPLOY ID", "STATUS");
        println!("{}", "-".repeat(100));

        for item in items {
            println!(
                "{:<36} {:<18} {:<10} {}",
                truncate(&item.project_name, 36),
                truncate(&item.id, 18),
                item.status,
                item.url.as_deref().unwrap_or("—")
            );
            println!("  deployed: {}", item.created_at);
        }

        Ok(())
    })
    .await
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        format!(
            "{}…",
            value
                .chars()
                .take(max.saturating_sub(1))
                .collect::<String>()
        )
    }
}
