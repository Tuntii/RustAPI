use rustapi_cloud::{build_app, config::Config, db::create_pool};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let config = Config::load();
    let pool = create_pool(&config.database_url).await;
    let addr = format!("{}:{}", config.host, config.port);

    tracing::info!("RustAPI Cloud starting on {}", addr);

    build_app(config, pool)
        .run(&addr)
        .await
        .expect("Server failed to start");
}