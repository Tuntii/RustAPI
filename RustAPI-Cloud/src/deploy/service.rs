use std::path::{Path, PathBuf};
use std::sync::Arc;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rustapi_rs::prelude::*;
use serde::Serialize;
use tokio::fs;

use crate::db::DbPool;
use crate::deploy::executor;
use crate::models::{Deploy, DeployStatus, NewDeploy, NewProject, Project};
use crate::schema::{deploys, projects};

#[derive(Debug, Clone, Serialize, Schema)]
pub struct DeployResponse {
    pub deploy_id: String,
    pub status: String,
    pub url: Option<String>,
}

#[derive(Clone)]
pub struct DeployService {
    pool: Arc<DbPool>,
    storage_root: PathBuf,
}

impl DeployService {
    pub fn new(pool: DbPool, storage_root: impl Into<PathBuf>) -> Self {
        Self {
            pool: Arc::new(pool),
            storage_root: storage_root.into(),
        }
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    pub fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    /// Parse multipart fields and persist a new deploy for the authenticated user.
    pub async fn create_from_multipart(
        &self,
        user_id: &str,
        mut multipart: Multipart,
    ) -> Result<DeployResponse> {
        let mut project_name: Option<String> = None;
        let mut binary_data: Option<Vec<u8>> = None;

        while let Some(field) = multipart.next_field().await? {
            match field.name() {
                Some("project_name") => {
                    project_name = Some(field.text().await?);
                }
                Some("binary") => {
                    binary_data = Some(field.bytes().await?.to_vec());
                }
                _ => {}
            }
        }

        let project_name = project_name
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| ApiError::bad_request("Missing project_name field"))?;

        let binary_data = binary_data
            .filter(|data| !data.is_empty())
            .ok_or_else(|| ApiError::bad_request("Missing binary field"))?;

        self.create_from_upload(user_id, &project_name, &binary_data)
            .await
    }

    /// Write binary to storage, insert project/deploy rows, and queue launch.
    pub async fn create_from_upload(
        &self,
        user_id: &str,
        project_name: &str,
        binary_data: &[u8],
    ) -> Result<DeployResponse> {
        let project = self.find_or_create_project(user_id, project_name).await?;
        let new_deploy = NewDeploy::new(&project.id, user_id, "");
        let deploy_id = new_deploy.id.clone();

        let deploy_dir = self
            .storage_root
            .join(user_id)
            .join(&project.id)
            .join(&deploy_id);

        fs::create_dir_all(&deploy_dir)
            .await
            .map_err(|err| ApiError::internal(format!("Failed to create storage dir: {}", err)))?;

        let binary_path = deploy_dir.join(format!("{}.bin", project_name));
        fs::write(&binary_path, binary_data)
            .await
            .map_err(|err| ApiError::internal(format!("Failed to write binary: {}", err)))?;

        let binary_path_str = binary_path.to_string_lossy().to_string();
        let new_deploy = NewDeploy {
            id: deploy_id.clone(),
            project_id: project.id.clone(),
            user_id: user_id.to_string(),
            binary_path: binary_path_str.clone(),
            status: DeployStatus::Queued.as_str().to_string(),
        };

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApiError::internal(format!("DB connection failed: {}", err)))?;

        diesel::insert_into(deploys::table)
            .values(&new_deploy)
            .execute(&mut conn)
            .await
            .map_err(|err| ApiError::internal(format!("Failed to insert deploy: {}", err)))?;

        let deploy_id = new_deploy.id.clone();
        let pool = self.pool.clone();
        let launch_path = binary_path_str.clone();
        let launch_id = deploy_id.clone();

        tokio::spawn(async move {
            executor::launch_binary(pool, launch_id, launch_path).await;
        });

        Ok(DeployResponse {
            deploy_id,
            status: DeployStatus::Queued.as_str().to_string(),
            url: None,
        })
    }

    /// Load deploy status for the given user.
    pub async fn get_status(&self, user_id: &str, deploy_id: &str) -> Result<DeployResponse> {
        let deploy = self.fetch_deploy(user_id, deploy_id).await?;
        Ok(DeployResponse {
            deploy_id: deploy.id,
            status: deploy.status,
            url: deploy.url,
        })
    }

    async fn fetch_deploy(&self, user_id: &str, deploy_id: &str) -> Result<Deploy> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApiError::internal(format!("DB connection failed: {}", err)))?;

        deploys::table
            .filter(deploys::id.eq(deploy_id))
            .filter(deploys::user_id.eq(user_id))
            .select(Deploy::as_select())
            .first(&mut conn)
            .await
            .optional()
            .map_err(|err| ApiError::internal(format!("DB query failed: {}", err)))?
            .ok_or_else(|| ApiError::not_found("Deploy not found"))
    }

    async fn find_or_create_project(&self, user_id: &str, name: &str) -> Result<Project> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| ApiError::internal(format!("DB connection failed: {}", err)))?;

        if let Some(project) = projects::table
            .filter(projects::user_id.eq(user_id))
            .filter(projects::name.eq(name))
            .select(Project::as_select())
            .first(&mut conn)
            .await
            .optional()
            .map_err(|err| ApiError::internal(format!("DB query failed: {}", err)))?
        {
            return Ok(project);
        }

        let new_project = NewProject::new(user_id, name);
        diesel::insert_into(projects::table)
            .values(&new_project)
            .execute(&mut conn)
            .await
            .map_err(|err| ApiError::internal(format!("Failed to insert project: {}", err)))?;

        projects::table
            .filter(projects::id.eq(&new_project.id))
            .select(Project::as_select())
            .first(&mut conn)
            .await
            .map_err(|err| ApiError::internal(format!("Failed to load project: {}", err)))
    }
}