use super::{JobBackend, JobRequest};
use crate::error::{JobError, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Pool, Postgres, Row};
use std::sync::Arc;

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

#[async_trait]
impl JobBackend for PostgresBackend {
    async fn push(&self, job: JobRequest) -> Result<()> {
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
    }

    async fn pop(&self) -> Result<Option<JobRequest>> {
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
            // Reconstruct JobRequest
            // Note: In a real persistent queue we wouldn't DELETE, we'd update status to 'processing'
            // and have a 'cleanup' or 'retry' mechanism for stalled jobs.
            // But staying consistent with simple queue contract for now.

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
    }

    async fn complete(&self, _job_id: &str) -> Result<()> {
        // Already deleted
        Ok(())
    }

    async fn fail(&self, _job_id: &str, _error: &str) -> Result<()> {
        // Already deleted. DLQ logic would go here.
        Ok(())
    }
}
