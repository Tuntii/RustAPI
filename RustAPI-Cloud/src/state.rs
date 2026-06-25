use crate::config::Config;
use crate::db::DbPool;
use crate::deploy::DeployService;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub deploy_service: DeployService,
}

impl AppState {
    pub fn new(config: Config, pool: DbPool) -> Self {
        let deploy_service = DeployService::new(pool, &config.storage_root);
        Self {
            config,
            deploy_service,
        }
    }
}