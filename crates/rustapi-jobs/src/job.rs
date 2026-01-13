use crate::error::Result;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

/// Context passed to job execution
#[derive(Debug, Clone)]
pub struct JobContext {
    pub job_id: String,
    pub attempt: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A job that can be executed
#[async_trait]
pub trait Job: Send + Sync + 'static {
    /// The job name/type
    const NAME: &'static str;

    /// The data required by the job
    type Data: Serialize + DeserializeOwned + Send + Sync + Debug;

    /// Execute the job
    async fn execute(&self, ctx: JobContext, data: Self::Data) -> Result<()>;
}

/// A type-erased job handler
#[async_trait]
pub trait JobHandler: Send + Sync {
    async fn handle(&self, ctx: JobContext, data: serde_json::Value) -> Result<()>;
}

#[async_trait]
impl<J: Job> JobHandler for J {
    async fn handle(&self, ctx: JobContext, data: serde_json::Value) -> Result<()> {
        let data: J::Data = serde_json::from_value(data)?;
        self.execute(ctx, data).await
    }
}
