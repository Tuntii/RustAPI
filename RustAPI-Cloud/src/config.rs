use dotenvy::dotenv;
use std::env;

/// Production deploy routing (public URLs + nginx map files for user apps).
#[derive(Clone, Debug, Default)]
pub struct DeploySettings {
    /// Base host for deployed apps, e.g. `rustapi.tunayinbayramharciligi.com`.
    /// Apps are served at `{project}-{user8}.{public_host}`.
    pub public_host: Option<String>,
    pub url_scheme: String,
    /// Directory of per-app `.conf` map snippets included by nginx wildcard vhost.
    pub nginx_map_dir: Option<String>,
}

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub github_redirect_uri: String,
    pub storage_root: String,
    pub deploy: DeploySettings,
}

impl Config {
    pub fn load() -> Self {
        let _ = dotenv();

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .expect("PORT must be a number"),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            github_client_id: env::var("GITHUB_CLIENT_ID").expect("GITHUB_CLIENT_ID must be set"),
            github_client_secret: env::var("GITHUB_CLIENT_SECRET")
                .expect("GITHUB_CLIENT_SECRET must be set"),
            github_redirect_uri: env::var("GITHUB_REDIRECT_URI")
                .expect("GITHUB_REDIRECT_URI must be set"),
            storage_root: env::var("STORAGE_ROOT").unwrap_or_else(|_| "./storage".into()),
            deploy: DeploySettings {
                public_host: env::var("DEPLOY_PUBLIC_HOST")
                    .ok()
                    .filter(|value| !value.trim().is_empty()),
                url_scheme: env::var("DEPLOY_URL_SCHEME").unwrap_or_else(|_| "https".into()),
                nginx_map_dir: env::var("NGINX_DEPLOY_MAP_DIR")
                    .ok()
                    .filter(|value| !value.trim().is_empty()),
            },
        }
    }
}
