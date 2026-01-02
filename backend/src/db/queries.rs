use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::path::Path;
use tracing::{error, info};

use crate::jobs::{Job, JobStage, JobStatus};

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct JobRecord {
    pub id: i64,
    pub disc_label: String,
    pub title: Option<String>,
    pub year: Option<i32>,
    pub status: String,
    pub progress: f64,
    pub stage: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RipRecord {
    pub id: i64,
    pub job_id: i64,
    pub disc_id: String,
    pub output_path: String,
    pub original_size: Option<i64>,
    pub final_size: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RipStats {
    pub total_rips: i64,
    pub total_original_size: i64,
    pub total_final_size: i64,
    pub storage_saved: i64,
}

impl Database {
    /// Create a new database connection and initialize schema
    pub async fn new(database_path: &str) -> Result<Self, sqlx::Error> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(database_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let pool = SqlitePool::connect(&format!("sqlite:{}", database_path)).await?;

        // Initialize schema
        let schema = include_str!("schema.sql");
        sqlx::query(schema).execute(&pool).await?;

        info!("Database initialized at {}", database_path);
        Ok(Self { pool })
    }

    /// Insert a new job
    pub async fn insert_job(&self, job: &Job) -> Result<i64, sqlx::Error> {
        let status_str = format!("{:?}", job.status);
        let stage_str = job.stage.as_ref().map(|s| format!("{:?}", s));

        let result = sqlx::query!(
            r#"
            INSERT INTO jobs (disc_label, title, year, status, progress, stage)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            job.disc_label,
            job.title,
            job.year,
            status_str,
            job.progress,
            stage_str
        )
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Update job status and progress
    pub async fn update_job(
        &self,
        id: i64,
        status: &JobStatus,
        progress: f64,
        stage: Option<&JobStage>,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let status_str = format!("{:?}", status);
        let stage_str = stage.map(|s| format!("{:?}", s));
        let completed_at = if matches!(status, JobStatus::Completed | JobStatus::Failed) {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query!(
            r#"
            UPDATE jobs
            SET status = ?, progress = ?, stage = ?, updated_at = CURRENT_TIMESTAMP,
                completed_at = ?, error = ?
            WHERE id = ?
            "#,
            status_str,
            progress,
            stage_str,
            completed_at,
            error,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get job by ID
    pub async fn get_job(&self, id: i64) -> Result<Option<JobRecord>, sqlx::Error> {
        let record = sqlx::query_as!(
            JobRecord,
            r#"
            SELECT id, disc_label, title, year, status, progress, stage,
                   created_at as "created_at: DateTime<Utc>",
                   updated_at as "updated_at: DateTime<Utc>",
                   completed_at as "completed_at: Option<DateTime<Utc>>",
                   error
            FROM jobs
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all jobs with optional status filter
    pub async fn get_jobs(
        &self,
        status_filter: Option<&str>,
    ) -> Result<Vec<JobRecord>, sqlx::Error> {
        let records = if let Some(status) = status_filter {
            sqlx::query_as!(
                JobRecord,
                r#"
                SELECT id, disc_label, title, year, status, progress, stage,
                       created_at as "created_at: DateTime<Utc>",
                       updated_at as "updated_at: DateTime<Utc>",
                       completed_at as "completed_at: Option<DateTime<Utc>>",
                       error
                FROM jobs
                WHERE status = ?
                ORDER BY created_at DESC
                "#,
                status
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as!(
                JobRecord,
                r#"
                SELECT id, disc_label, title, year, status, progress, stage,
                       created_at as "created_at: DateTime<Utc>",
                       updated_at as "updated_at: DateTime<Utc>",
                       completed_at as "completed_at: Option<DateTime<Utc>>",
                       error
                FROM jobs
                ORDER BY created_at DESC
                "#
            )
            .fetch_all(&self.pool)
            .await?
        };

        Ok(records)
    }

    /// Delete a job
    pub async fn delete_job(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM jobs WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Insert a completed rip record
    pub async fn insert_rip(
        &self,
        job_id: i64,
        disc_id: &str,
        output_path: &str,
        original_size: Option<i64>,
        final_size: Option<i64>,
        duration_seconds: Option<i64>,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            INSERT INTO rips (job_id, disc_id, output_path, original_size, final_size, duration_seconds)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            job_id,
            disc_id,
            output_path,
            original_size,
            final_size,
            duration_seconds
        )
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Check if disc has been ripped before
    pub async fn is_duplicate(&self, disc_id: &str) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count: i64"
            FROM rips
            WHERE disc_id = ?
            "#,
            disc_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Get rip history with pagination
    pub async fn get_rip_history(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RipRecord>, sqlx::Error> {
        let records = sqlx::query_as!(
            RipRecord,
            r#"
            SELECT id, job_id, disc_id, output_path, original_size, final_size,
                   duration_seconds, completed_at as "completed_at: DateTime<Utc>"
            FROM rips
            ORDER BY completed_at DESC
            LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get rip statistics
    pub async fn get_rip_stats(&self) -> Result<RipStats, sqlx::Error> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as "total_rips: i64",
                COALESCE(SUM(original_size), 0) as "total_original_size: i64",
                COALESCE(SUM(final_size), 0) as "total_final_size: i64"
            FROM rips
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let storage_saved = stats.total_original_size - stats.total_final_size;

        Ok(RipStats {
            total_rips: stats.total_rips,
            total_original_size: stats.total_original_size,
            total_final_size: stats.total_final_size,
            storage_saved,
        })
    }

    /// Get config value
    pub async fn get_config(&self, key: &str) -> Result<Option<String>, sqlx::Error> {
        let value = sqlx::query_scalar!(
            r#"
            SELECT value
            FROM config
            WHERE key = ?
            "#,
            key
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(value)
    }

    /// Set config value
    pub async fn set_config(&self, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO config (key, value, updated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET value = ?, updated_at = CURRENT_TIMESTAMP
            "#,
            key,
            value,
            value
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all config as JSON
    pub async fn get_all_config(&self) -> Result<serde_json::Value, sqlx::Error> {
        let records = sqlx::query!("SELECT key, value FROM config")
            .fetch_all(&self.pool)
            .await?;

        let mut config = serde_json::Map::new();
        for record in records {
            if let Ok(value) = serde_json::from_str(&record.value) {
                config.insert(record.key, value);
            }
        }

        Ok(serde_json::Value::Object(config))
    }
}
