//! Shared Postgres bootstrap for verification binaries.
use std::path::PathBuf;
use std::time::Duration;

use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V13};
use pg_embed::postgres::{PgEmbed, PgSettings};
use tokio_postgres::NoTls;

pub async fn embedded_database_url() -> anyhow::Result<String> {
    let data_dir = std::env::temp_dir().join(format!(
        "rustapi-cloud-verify-pg-{}",
        std::process::id()
    ));
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        port
    };
    let pg_settings = PgSettings {
        database_dir: data_dir,
        port,
        user: "postgres".into(),
        password: "password".into(),
        auth_method: PgAuthMethod::Plain,
        persistent: false,
        timeout: Some(Duration::from_secs(60)),
        migration_dir: None,
    };
    let fetch_settings = PgFetchSettings {
        version: PG_V13,
        ..Default::default()
    };
    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;
    pg.setup().await?;
    pg.start_db().await?;
    let url = pg.db_uri.clone();
    apply_migrations(&url).await?;
    std::mem::forget(pg);
    Ok(url)
}

pub async fn dump_schema(database_url: &str) -> anyhow::Result<String> {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let rows = client
        .query(
            "SELECT table_name, column_name, data_type \
             FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name IN ('projects', 'deploys') \
             ORDER BY table_name, ordinal_position",
            &[],
        )
        .await?;
    let mut out = String::from("-- schema dump\n");
    for row in rows {
        let table: String = row.get(0);
        let column: String = row.get(1);
        let dtype: String = row.get(2);
        out.push_str(&format!("  {}.{} ({})\n", table, column, dtype));
    }
    Ok(out)
}

async fn apply_migrations(database_url: &str) -> anyhow::Result<()> {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for file in ["001_init.sql", "002_deploy.sql"] {
        let sql = std::fs::read_to_string(root.join(file))?;
        client.batch_execute(&sql).await?;
    }
    Ok(())
}