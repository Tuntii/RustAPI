use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::process::Command;
use tracing::{error, info, warn};

use crate::config::DeploySettings;
use crate::db::DbPool;
use crate::deploy::routing;
use crate::models::DeployStatus;

static PORT_CURSOR: AtomicU16 = AtomicU16::new(30_000);

#[derive(Clone, Debug)]
pub struct LaunchParams {
    pub deploy_id: String,
    pub binary_path: String,
    pub project_name: String,
    pub user_id: String,
    pub deploy: DeploySettings,
}

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

fn resolve_public_url(port: u16, params: &LaunchParams) -> String {
    if let Some(public_host) = params.deploy.public_host.as_deref() {
        routing::public_deploy_url(
            &params.deploy.url_scheme,
            public_host,
            &params.project_name,
            &params.user_id,
        )
    } else {
        format!("http://127.0.0.1:{port}")
    }
}

fn register_nginx_route(port: u16, params: &LaunchParams) {
    let (Some(public_host), Some(map_dir)) = (
        params.deploy.public_host.as_deref(),
        params.deploy.nginx_map_dir.as_deref(),
    ) else {
        return;
    };

    let hostname = routing::deploy_hostname(public_host, &params.project_name, &params.user_id);
    let map_path = PathBuf::from(map_dir);
    if let Err(err) = routing::upsert_nginx_map_entry(&map_path, &hostname, port) {
        warn!(
            deploy_id = %params.deploy_id,
            error = %err,
            "failed to write nginx deploy map entry"
        );
        return;
    }
    if let Err(err) = routing::reload_nginx() {
        warn!(
            deploy_id = %params.deploy_id,
            error = %err,
            "nginx reload skipped or failed"
        );
    }
}

pub async fn launch_binary(pool: Arc<DbPool>, params: LaunchParams) {
    let path = Path::new(&params.binary_path);
    if !path.exists() {
        mark_failed(&pool, &params.deploy_id, "Binary file not found").await;
        return;
    }

    if let Err(err) = make_executable(path) {
        mark_failed(&pool, &params.deploy_id, &format!("chmod failed: {}", err)).await;
        return;
    }

    let port = allocate_port();
    let url = resolve_public_url(port, &params);

    let mut command = Command::new(path);
    command.env("PORT", port.to_string());
    command.kill_on_drop(false);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            mark_failed(&pool, &params.deploy_id, &format!("spawn failed: {}", err)).await;
            return;
        }
    };

    let pid = child.id().map(|id| id as i32);

    if let Err(err) = update_running(&pool, &params.deploy_id, port, &url, pid).await {
        error!(deploy_id = %params.deploy_id, error = %err, "failed to update deploy to running");
        return;
    }

    info!(deploy_id = %params.deploy_id, port = port, "deploy process spawned, waiting for bind");

    if !wait_for_port_ready(port, Duration::from_secs(10)).await {
        mark_failed(
            &pool,
            &params.deploy_id,
            "process did not bind to port in time",
        )
        .await;
        return;
    }

    register_nginx_route(port, &params);

    if let Err(err) = mark_live(&pool, &params.deploy_id).await {
        error!(deploy_id = %params.deploy_id, error = %err, "failed to mark deploy live");
        return;
    }

    info!(deploy_id = %params.deploy_id, port = port, url = %url, "deploy is live");
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
    use crate::config::DeploySettings;

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

    #[test]
    fn resolve_public_url_uses_deploy_host_when_configured() {
        let params = LaunchParams {
            deploy_id: "d".into(),
            binary_path: "/tmp/x".into(),
            project_name: "my-api".into(),
            user_id: "a5c231e4-4bbd-4c99-8993-771aacd3dd35".into(),
            deploy: DeploySettings {
                public_host: Some("rustapi.tunayinbayramharciligi.com".into()),
                url_scheme: "https".into(),
                nginx_map_dir: None,
            },
        };
        let url = resolve_public_url(30123, &params);
        assert_eq!(
            url,
            "https://my-api-a5c231e4.rustapi.tunayinbayramharciligi.com"
        );
    }

    #[test]
    fn resolve_public_url_falls_back_to_localhost_in_dev() {
        let params = LaunchParams {
            deploy_id: "d".into(),
            binary_path: "/tmp/x".into(),
            project_name: "app".into(),
            user_id: "user".into(),
            deploy: DeploySettings::default(),
        };
        assert_eq!(resolve_public_url(30123, &params), "http://127.0.0.1:30123");
    }
}