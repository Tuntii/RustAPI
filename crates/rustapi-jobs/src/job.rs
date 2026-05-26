use crate::error::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

/// Context passed to job execution
#[derive(Debug, Clone)]
pub struct JobContext {
    pub job_id: String,
    pub attempt: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A job that can be executed
pub trait Job: Send + Sync + 'static {
    /// The job name/type
    const NAME: &'static str;

    /// The data required by the job
    type Data: Serialize + DeserializeOwned + Send + Sync + Debug;

    /// Execute the job
    fn execute(&self, ctx: JobContext, data: Self::Data)
        -> impl Future<Output = Result<()>> + Send;
}

/// A type-erased job handler (dyn-compatible via boxed futures)
pub trait JobHandler: Send + Sync {
    fn handle<'a>(
        &'a self,
        ctx: JobContext,
        data: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

impl<J: Job> JobHandler for J {
    fn handle<'a>(
        &'a self,
        ctx: JobContext,
        data: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let data: J::Data = serde_json::from_value(data)?;
            self.execute(ctx, data).await
        })
    }
}
