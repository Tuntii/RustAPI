use super::{JobBackend, JobRequest};
use crate::error::{JobError, Result};
use async_trait::async_trait;
use redis::{AsyncCommands, Client, Script};
use std::sync::Arc;
use tokio::sync::Mutex;

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

#[async_trait]
impl JobBackend for RedisBackend {
    async fn push(&self, job: JobRequest) -> Result<()> {
        let mut conn = self
            .client
            .get_async_connection()
            .await
            .map_err(|e| JobError::BackendError(e.to_string()))?;

        let score = job.run_at.unwrap_or(chrono::Utc::now()).timestamp() as f64;
        let payload = serde_json::to_string(&job)?;

        conn.zadd(&self.queue_key, score, payload)
            .await
            .map_err(|e| JobError::BackendError(e.to_string()))?;

        Ok(())
    }

    async fn pop(&self) -> Result<Option<JobRequest>> {
        let mut conn = self
            .client
            .get_async_connection()
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
    }

    async fn complete(&self, _job_id: &str) -> Result<()> {
        // Job is already removed from ZSET on pop
        // In a reliable system we would move it to a 'processing' set first
        // But for this implementation we assume 'at-most-once' or simple mechanics
        Ok(())
    }

    async fn fail(&self, _job_id: &str, _error: &str) -> Result<()> {
        // Similar to complete, it's already removed.
        // We could implement a DLQ here.
        Ok(())
    }
}
