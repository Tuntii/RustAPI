use super::{JobBackend, JobRequest};
use super::super::error::{JobError, Result};
use redis::{AsyncCommands, Client, Script};
use std::future::Future;
use std::pin::Pin;

/// Redis-backed job queue
#[derive(Debug, Clone)]
pub struct RedisBackend {
    client: Client,
    queue_key: String,
    pop_script: Script,
}

impl RedisBackend {
    pub fn new(url: &str, queue_key: &str) -> Result<Self> {
        let client = Client::open(url).map_err(|e| JobError::ConfigError(e.to_string()))?;

        // Lua script to atomically pop the first ready job
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

            let score = job
                .run_at
                .unwrap_or(chrono::Utc::now())
                .timestamp() as f64;
            let payload = serde_json::to_string(&job)?;

            conn.zadd::<_, _, _, ()>(&self.queue_key, score, payload)
                .await
                .map_err(|e| JobError::BackendError(e.to_string()))?;

            Ok(())
        })
    }

    fn pop<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<JobRequest>>> + Send + 'a>> {
        Box::pin(async move {
            let mut conn = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| JobError::BackendError(e.to_string()))?;

            let now = chrono::Utc::now().timestamp();

            let result: Option<String> = self
                .pop_script
                .key(&self.queue_key)
                .arg(now)
                .invoke_async(&mut conn)
                .await
                .map_err(|e| JobError::BackendError(e.to_string()))?;

            match result {
                Some(payload) => {
                    let job: JobRequest = serde_json::from_str(&payload)?;
                    Ok(Some(job))
                }
                None => Ok(None),
            }
        })
    }

    fn complete<'a>(
        &'a self,
        _job_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        // Jobs are removed on pop in Redis ZSET
        Box::pin(async move { Ok(()) })
    }

    fn fail<'a>(
        &'a self,
        _job_id: &'a str,
        _error: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }
}
