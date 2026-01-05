//! Docs command - open API documentation

use anyhow::Result;
use console::style;

/// Open API documentation in browser
pub async fn open_docs(port: u16) -> Result<()> {
    let url = format!("http://localhost:{}/docs", port);

    println!("Opening {} in browser...", style(&url).cyan());

    // Try to open in browser
    #[cfg(target_os = "windows")]
    {
        tokio::process::Command::new("cmd")
            .args(["/C", "start", &url])
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        tokio::process::Command::new("open").arg(&url).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        tokio::process::Command::new("xdg-open").arg(&url).spawn()?;
    }

    println!();
    println!(
        "{}",
        style("Make sure your RustAPI server is running!").yellow()
    );
    println!("Start it with: {}", style("cargo rustapi run").cyan());

    Ok(())
}
