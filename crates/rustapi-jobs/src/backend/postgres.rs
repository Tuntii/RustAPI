use super::{JobBackend, JobRequest};
use crate::error::{JobError, Result};
use sqlx::{Pool, Postgres, Row};
use std::future::Future;
use std::pin::Pin;

/// Postgres-backed job queue
#[derive(Debug, Clone)]
pub struct PostgresBackend {
    pool: Pool<Postgres>,
    table_name: String,
}

impl PostgresBackend {
    pub fn new(pool: Pool<Postgres>, table_name: &str) -> Self {
        Self {
            pool,
            table_name: table_name.to_string(),
        }
    }

    /// Initialize the database schema
    pub async fn ensure_schema(&self) -> Result<()> {
        let query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                payload JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                run_at TIMESTAMPTZ,
                attempts INT DEFAULT 0,
                max_attempts INT DEFAULT 3,
                last_error TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_{}_run_at ON {} (run_at);
            "#,
            self.table_name, self.table_name, self.table_name
        );

        sqlx::query(&query)
            .execute(&self.pool)
            .await
            .map_err(|e| JobError::BackendError(e.to_string()))?;

        Ok(())
    }
}

impl JobBackend for PostgresBackend {
    fn push<'a>(
        &'a self,
        job: JobRequest,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let query = format!(
                r#"
            INSERT INTO {} (id, name, payload, created_at, run_at, attempts, max_attempts, last_error)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
                self.table_name
            );

            sqlx::query(&query)
                .bind(&job.id)
                .bind(&job.name)
                .bind(&job.payload)
                .bind(job.created_at)
                .bind(job.run_at)
                .bind(job.attempts as i32)
                .bind(job.max_attempts as i32)
                .bind(&job.last_error)
                .execute(&self.pool)
                .await
                .map_err(|e| JobError::BackendError(e.to_string()))?;

            Ok(())
        })
    }

    fn pop<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<JobRequest>>> + Send + 'a>> {
        Box::pin(async move {
            // Atomic pop using DELETE ... RETURNING with locking
            let query = format!(
                r#"
            DELETE FROM {}
            WHERE id = (
                SELECT id
                FROM {}
                WHERE (run_at IS NULL OR run_at <= NOW())
                ORDER BY run_at ASC, created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING id, name, payload, created_at, run_at, attempts, max_attempts, last_error
            "#,
                self.table_name, self.table_name
            );

            let row = sqlx::query(&query)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| JobError::BackendError(e.to_string()))?;

            if let Some(row) = row {
                Ok(Some(JobRequest {
                    id: row.get("id"),
                    name: row.get("name"),
                    payload: row.get("payload"),
                    created_at: row.get("created_at"),
                    run_at: row.get("run_at"),
                    attempts: row.get::<i32, _>("attempts") as u32,
                    max_attempts: row.get::<i32, _>("max_attempts") as u32,
                    last_error: row.get("last_error"),
                }))
            } else {
                Ok(None)
            }
        })
    }

    fn complete<'a>(
        &'a self,
        _job_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        // Already deleted
        Box::pin(async move { Ok(()) })
    }

    fn fail<'a>(
        &'a self,
        _job_id: &'a str,
        _error: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        // Already deleted. DLQ logic would go here.
        Box::pin(async move { Ok(()) })
    }
}
