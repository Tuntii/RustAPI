use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V13};
use pg_embed::postgres::{PgEmbed, PgSettings};
use tokio::sync::OnceCell;
use tokio_postgres::NoTls;

static EMBEDDED_PG: OnceCell<Arc<tokio::sync::Mutex<PgEmbed>>> = OnceCell::const_new();
static DATABASE_URL: OnceCell<String> = OnceCell::const_new();

pub const JWT_SECRET: &str = "test-jwt-secret-deploy-pipeline";

pub async fn database_url() -> String {
    DATABASE_URL
        .get_or_init(|| async {
            let data_dir = std::env::temp_dir().join(format!(
                "rustapi-cloud-pg-embed-{}",
                std::process::id()
            ));
            let port = {
                let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
                let port = listener.local_addr().expect("addr").port();
                drop(listener);
                port
            };
            let pg_settings = PgSettings {
                database_dir: data_dir,
                port,
                user: "postgres".to_string(),
                password: "password".to_string(),
                auth_method: PgAuthMethod::Plain,
                persistent: false,
                timeout: Some(Duration::from_secs(30)),
                migration_dir: None,
            };
            let fetch_settings = PgFetchSettings {
                version: PG_V13,
                ..Default::default()
            };
            let mut pg = PgEmbed::new(pg_settings, fetch_settings)
                .await
                .expect("pg embed");
            pg.setup().await.expect("pg setup");
            pg.start_db().await.expect("pg start");
            let url = pg.db_uri.clone();
            apply_migrations(&url).await;
            let _ = EMBEDDED_PG.set(Arc::new(tokio::sync::Mutex::new(pg)));
            url
        })
        .await
        .clone()
}

pub async fn dump_deploy_schema() -> String {
    let url = database_url().await;
    let (client, connection) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("connect for schema dump");
    tokio::spawn(async move {
        if let Err(err) = connection.await {
            panic!("schema dump connection error: {}", err);
        }
    });

    let rows = client
        .query(
            "SELECT table_name, column_name, data_type \
             FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name IN ('projects', 'deploys') \
             ORDER BY table_name, ordinal_position",
            &[],
        )
        .await
        .expect("schema query");

    let mut out = String::from("projects + deploys schema:\n");
    for row in rows {
        let table: String = row.get(0);
        let column: String = row.get(1);
        let dtype: String = row.get(2);
        out.push_str(&format!("  {}.{} ({})\n", table, column, dtype));
    }
    out
}

async fn apply_migrations(database_url: &str) {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls)
        .await
        .expect("connect for migrations");
    tokio::spawn(async move {
        if let Err(err) = connection.await {
            panic!("migration connection error: {}", err);
        }
    });

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for file in ["001_init.sql", "002_deploy.sql"] {
        let sql = std::fs::read_to_string(root.join(file)).expect("read migration");
        client.batch_execute(&sql).await.expect("apply migration");
    }
}