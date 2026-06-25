use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::process::Command;
use tracing::{error, info};

use crate::db::DbPool;
use crate::models::DeployStatus;

static PORT_CURSOR: AtomicU16 = AtomicU16::new(30_000);

/// Reserve a free localhost port by binding ephemeral then releasing.
/// Caller must verify the child process binds before marking live.
pub fn allocate_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("failed to bind ephemeral port");
    let port = listener.local_addr().expect("local_addr").port();
    drop(listener);
    PORT_CURSOR.store(port.wrapping_add(1), Ordering::Relaxed);
    port
}

/// Poll until the port accepts TCP connections or timeout.
pub async fn wait_for_port_ready(port: u16, timeout: Duration) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Make a binary executable on Unix hosts.
pub fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

pub async fn launch_binary(pool: Arc<DbPool>, deploy_id: String, binary_path: String) {
    let path = Path::new(&binary_path);
    if !path.exists() {
        mark_failed(&pool, &deploy_id, "Binary file not found").await;
        return;
    }

    if let Err(err) = make_executable(path) {
        mark_failed(&pool, &deploy_id, &format!("chmod failed: {}", err)).await;
        return;
    }

    let port = allocate_port();
    let url = format!("http://127.0.0.1:{}", port);

    let mut command = Command::new(path);
    command.env("PORT", port.to_string());
    command.kill_on_drop(false);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            mark_failed(&pool, &deploy_id, &format!("spawn failed: {}", err)).await;
            return;
        }
    };

    let pid = child.id().map(|id| id as i32);

    if let Err(err) = update_running(&pool, &deploy_id, port, &url, pid).await {
        error!(deploy_id = %deploy_id, error = %err, "failed to update deploy to running");
        return;
    }

    info!(deploy_id = %deploy_id, port = port, "deploy process spawned, waiting for bind");

    if !wait_for_port_ready(port, Duration::from_secs(10)).await {
        mark_failed(&pool, &deploy_id, "process did not bind to port in time").await;
        return;
    }

    if let Err(err) = mark_live(&pool, &deploy_id).await {
        error!(deploy_id = %deploy_id, error = %err, "failed to mark deploy live");
        return;
    }

    info!(deploy_id = %deploy_id, port = port, url = %url, "deploy is live");
    tokio::spawn(async move {
        let _ = child.wait().await;
    });
}

async fn update_running(
    pool: &DbPool,
    deploy_id: &str,
    port: u16,
    url: &str,
    pid: Option<i32>,
) -> anyhow::Result<()> {
    use chrono::Utc;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    use crate::schema::deploys;

    let mut conn = pool.get().await?;
    diesel::update(deploys::table.filter(deploys::id.eq(deploy_id)))
        .set((
            deploys::status.eq(DeployStatus::Running.as_str()),
            deploys::port.eq(Some(port as i32)),
            deploys::url.eq(Some(url.to_string())),
            deploys::pid.eq(pid),
            deploys::updated_at.eq(Utc::now().naive_utc()),
        ))
        .execute(&mut conn)
        .await?;
    Ok(())
}

async fn mark_live(pool: &DbPool, deploy_id: &str) -> anyhow::Result<()> {
    use chrono::Utc;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    use crate::schema::deploys;

    let mut conn = pool.get().await?;
    diesel::update(deploys::table.filter(deploys::id.eq(deploy_id)))
        .set((
            deploys::status.eq(DeployStatus::Live.as_str()),
            deploys::updated_at.eq(Utc::now().naive_utc()),
        ))
        .execute(&mut conn)
        .await?;
    Ok(())
}

async fn mark_failed(pool: &DbPool, deploy_id: &str, message: &str) {
    use chrono::Utc;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    use crate::schema::deploys;

    if let Ok(mut conn) = pool.get().await {
        let _ = diesel::update(deploys::table.filter(deploys::id.eq(deploy_id)))
            .set((
                deploys::status.eq(DeployStatus::Failed.as_str()),
                deploys::error_message.eq(Some(message.to_string())),
                deploys::updated_at.eq(Utc::now().naive_utc()),
            ))
            .execute(&mut conn)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_port_returns_ephemeral_port() {
        let port = allocate_port();
        assert!(port > 0);
        assert!(TcpListener::bind(("127.0.0.1", port)).is_ok());
    }

    #[tokio::test]
    async fn wait_for_port_ready_detects_listener() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((_, _)) = listener.accept() {}
        });
        assert!(wait_for_port_ready(port, Duration::from_secs(2)).await);
    }
}