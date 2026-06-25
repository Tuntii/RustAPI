pub mod verify_db;

pub mod auth;
pub mod config;
pub mod db;
pub mod deploy;
pub mod models;
pub mod routes;
pub mod schema;
pub mod state;

pub fn build_app(config: config::Config, pool: db::DbPool) -> rustapi_rs::RustApi {
    use rustapi_rs::prelude::*;

    let app_state = state::AppState::new(config.clone(), pool.clone());

    RustApi::new()
        .state(pool)
        .state(config)
        .state(app_state)
        .route("/health", get(routes::auth::health))
        .route("/auth/device", post(routes::auth::auth_device))
        .route("/auth/activate", get(routes::auth::auth_activate))
        .route("/auth/callback", get(routes::auth::auth_callback))
        .route("/auth/token", post(routes::auth::auth_token))
        .route("/auth/whoami", post(routes::auth::auth_whoami))
        .route("/deploy/:id/status", get(routes::deploy::deploy_status))
        .route("/deploy", post(routes::deploy::deploy_create))
}