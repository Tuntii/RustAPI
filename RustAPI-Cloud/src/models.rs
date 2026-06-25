use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: String,
    pub github_id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    pub tier: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser {
    pub id: String,
    pub github_id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    pub tier: String,
}

impl NewUser {
    pub fn from_github(
        github_id: i64,
        login: String,
        avatar_url: Option<String>,
        email: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            github_id,
            login,
            avatar_url,
            email,
            tier: "hobby".into(),
        }
    }
}

// ---- OAuth Device ----

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::oauth_devices)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OauthDevice {
    pub id: String,
    pub device_code: String,
    pub user_code: String,
    pub user_id: Option<String>,
    pub client_id: String,
    pub scopes: String,
    pub expires_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::oauth_devices)]
pub struct NewOauthDevice {
    pub id: String,
    pub device_code: String,
    pub user_code: String,
    pub client_id: String,
    pub scopes: String,
    pub expires_at: NaiveDateTime,
}

impl NewOauthDevice {
    pub fn new() -> Self {
        let device_code = uuid::Uuid::new_v4().to_string();
        let user_code = generate_user_code();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            device_code,
            user_code,
            client_id: "rustapi-cli".into(),
            scopes: "user:read".into(),
            expires_at: Utc::now().naive_utc() + chrono::Duration::minutes(15),
        }
    }

    pub fn device_code(&self) -> &str {
        &self.device_code
    }

    pub fn user_code(&self) -> &str {
        &self.user_code
    }
}

// ---- Project ----

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::projects)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Project {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::projects)]
pub struct NewProject {
    pub id: String,
    pub user_id: String,
    pub name: String,
}

impl NewProject {
    pub fn new(user_id: &str, name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
        }
    }
}

// ---- Deploy ----

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployStatus {
    Queued,
    Running,
    Live,
    Failed,
}

impl DeployStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Live => "live",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "running" => Self::Running,
            "live" => Self::Live,
            "failed" => Self::Failed,
            _ => Self::Queued,
        }
    }
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::deploys)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Deploy {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub binary_path: String,
    pub status: String,
    pub url: Option<String>,
    pub port: Option<i32>,
    pub pid: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::deploys)]
pub struct NewDeploy {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub binary_path: String,
    pub status: String,
}

impl NewDeploy {
    pub fn new(project_id: &str, user_id: &str, binary_path: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            user_id: user_id.to_string(),
            binary_path: binary_path.to_string(),
            status: DeployStatus::Queued.as_str().to_string(),
        }
    }
}

fn generate_user_code() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    uuid::Uuid::new_v4().hash(&mut hasher);
    let hash = hasher.finish();
    let code = format!("{:04X}-{:04X}", (hash >> 32) as u32, hash as u32);
    code
}
