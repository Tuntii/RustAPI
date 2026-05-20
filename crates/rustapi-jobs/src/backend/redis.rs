use super::{JobBackend, JobRequest};
use crate::error::{JobError, Result};
use redis::{AsyncCommands, Client, Script};
use std::future::Future;
use std::pin::Pin;

/// Redis-backed job queue
#[derive(Debug, Clone)]
pub struct RedisBackend {
    client: Client,
    queue_key: String,
    // Script is cheap to clone (Arc internal) or re-create
    pop_script: Script,
}

impl RedisBackend {
    pub fn new(url: &str, queue_key: &str) -> Result<Self> {
        let client = Client::open(url).map_err(|e| JobError::ConfigError(e.to_string()))?;

        // Lua script to atomically pop the first ready job
        // ZRANGEBYSCORE key -inf now LIMIT 0 1
        let pop_script = Script::new(
            r#"
            local jobs = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1], 'LIMIT', 0, 1)
            if #jobs > 0 then
                redis.call('ZREM', KEYS[1], jobs[1])
                return jobs[1]
            else
                return nil
            end
        "#,
        );

        Ok(Self {
            client,
            queue_key: queue_key.to_string(),
            pop_script,
        })
    }
}

impl JobBackend for RedisBackend {
    fn push<'a>(
        &'a self,
        job: JobRequest,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut conn = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| JobError::BackendError(e.to_string()))?;

            let score = job.run_at.unwrap_or(chrono::Utc::now()).timestamp() as f64;
            let payload = serde_json::to_string(&job)?;

            conn.zadd::<_, _, _, ()>(&self.queue_key, score, payload)
                .await
                .map_err(|e| JobError::BackendError(e.to_string()))?;

            Ok(())
        })
    }

    fn pop<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<Option<JobRequest>>> + Send + 'a>> {
        Box::pin(async move {
            let mut conn = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| JobError::BackendError(e.to_string()))?;

            let now = chrono::Utc::now().timestamp() as f64;

            let result: Option<String> = self
                .pop_script
                .key(&self.queue_key)
                .arg(now)
                .invoke_async(&mut conn)
                .await
                .map_err(|e| JobError::BackendError(e.to_string()))?;

            if let Some(json_str) = result {
                let job: JobRequest = serde_json::from_str(&json_str)?;
                Ok(Some(job))
            } else {
                Ok(None)
            }
        })
    }

    fn complete<'a>(
        &'a self,
        _job_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        // Job is already removed from ZSET on pop
        Box::pin(async move { Ok(()) })
    }

    fn fail<'a>(
        &'a self,
        _job_id: &'a str,
        _error: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        // Already removed. DLQ logic would go here.
        Box::pin(async move { Ok(()) })
    }
}
