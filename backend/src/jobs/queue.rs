use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};

use crate::api::WebSocketEvent;
use crate::db::Database;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Queued,
    Ripping,
    Transcoding,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStage {
    Detecting,
    Ripping,
    Transcoding,
    Finalizing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Option<i64>,
    pub disc_label: String,
    pub title: Option<String>,
    pub year: Option<i32>,
    pub status: JobStatus,
    pub progress: f64,
    pub stage: Option<JobStage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub output_path: Option<String>,
}

impl Job {
    pub fn new(disc_label: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            disc_label,
            title: None,
            year: None,
            status: JobStatus::Queued,
            progress: 0.0,
            stage: Some(JobStage::Detecting),
            created_at: now,
            updated_at: now,
            completed_at: None,
            error: None,
            output_path: None,
        }
    }

    pub fn with_metadata(mut self, title: String, year: Option<i32>) -> Self {
        self.title = Some(title);
        self.year = year;
        self
    }

    pub fn update_progress(&mut self, progress: f64, stage: JobStage) {
        self.progress = progress.min(100.0).max(0.0);
        self.stage = Some(stage);
        self.updated_at = Utc::now();
    }

    pub fn complete(&mut self, output_path: String) {
        self.status = JobStatus::Completed;
        self.progress = 100.0;
        self.completed_at = Some(Utc::now());
        self.output_path = Some(output_path);
    }

    pub fn fail(&mut self, error: String) {
        self.status = JobStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(Utc::now());
    }
}

#[derive(Clone)]
pub struct JobQueue {
    queue: Arc<RwLock<VecDeque<Job>>>,
    database: Database,
    event_sender: broadcast::Sender<WebSocketEvent>,
}

impl JobQueue {
    pub fn new(database: Database, event_sender: broadcast::Sender<WebSocketEvent>) -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            database,
            event_sender,
        }
    }

    /// Enqueue a new job
    pub async fn enqueue(&self, mut job: Job) -> Result<i64, String> {
        // Insert into database first
        let job_id = self
            .database
            .insert_job(&job)
            .await
            .map_err(|e| format!("Failed to insert job into database: {}", e))?;

        job.id = Some(job_id);

        // Add to in-memory queue
        let mut queue = self.queue.write().await;
        queue.push_back(job.clone());

        info!(
            "Job {} enqueued: {} ({})",
            job_id,
            job.disc_label,
            job.title.as_deref().unwrap_or("Unknown")
        );

        // Emit event
        let _ = self.event_sender.send(WebSocketEvent::JobStarted {
            job_id,
            disc_label: job.disc_label.clone(),
            title: job.title.clone(),
            year: job.year,
        });

        Ok(job_id)
    }

    /// Dequeue the next job
    pub async fn dequeue(&self) -> Option<Job> {
        let mut queue = self.queue.write().await;
        let job = queue.pop_front();

        if let Some(ref j) = job {
            debug!(
                "Job {} dequeued: {}",
                j.id.unwrap_or(-1),
                j.disc_label
            );
        }

        job
    }

    /// Get current queue length
    pub async fn len(&self) -> usize {
        let queue = self.queue.read().await;
        queue.len()
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.read().await;
        queue.is_empty()
    }

    /// Get all jobs in queue
    pub async fn get_all(&self) -> Vec<Job> {
        let queue = self.queue.read().await;
        queue.iter().cloned().collect()
    }

    /// Update job status in database and emit event
    pub async fn update_job_status(
        &self,
        job_id: i64,
        status: JobStatus,
        progress: f64,
        stage: Option<JobStage>,
    ) -> Result<(), String> {
        self.database
            .update_job(job_id, &status, progress, stage.as_ref(), None)
            .await
            .map_err(|e| format!("Failed to update job: {}", e))?;

        // Emit progress event
        let _ = self.event_sender.send(WebSocketEvent::JobProgress {
            job_id,
            progress,
            stage: stage.clone(),
            eta: None,
        });

        Ok(())
    }

    /// Mark job as completed
    pub async fn complete_job(
        &self,
        job_id: i64,
        output_path: String,
    ) -> Result<(), String> {
        self.database
            .update_job(
                job_id,
                &JobStatus::Completed,
                100.0,
                Some(&JobStage::Finalizing),
                None,
            )
            .await
            .map_err(|e| format!("Failed to complete job: {}", e))?;

        info!("Job {} completed: {}", job_id, output_path);

        // Emit completion event
        let _ = self.event_sender.send(WebSocketEvent::JobCompleted {
            job_id,
            output_path: output_path.clone(),
        });

        Ok(())
    }

    /// Mark job as failed
    pub async fn fail_job(&self, job_id: i64, error: String) -> Result<(), String> {
        self.database
            .update_job(
                job_id,
                &JobStatus::Failed,
                0.0,
                None,
                Some(&error),
            )
            .await
            .map_err(|e| format!("Failed to mark job as failed: {}", e))?;

        warn!("Job {} failed: {}", job_id, error);

        // Emit failure event
        let _ = self.event_sender.send(WebSocketEvent::JobFailed {
            job_id,
            error: error.clone(),
        });

        Ok(())
    }

    /// Remove job from queue (for cancellation)
    pub async fn remove_job(&self, job_id: i64) -> Result<(), String> {
        let mut queue = self.queue.write().await;
        queue.retain(|job| job.id != Some(job_id));

        self.database
            .delete_job(job_id)
            .await
            .map_err(|e| format!("Failed to delete job: {}", e))?;

        info!("Job {} removed from queue", job_id);

        Ok(())
    }

    /// Check if a disc has been ripped before
    pub async fn is_duplicate(&self, disc_id: &str) -> Result<bool, String> {
        self.database
            .is_duplicate(disc_id)
            .await
            .map_err(|e| format!("Failed to check for duplicate: {}", e))
    }

    /// Record completed rip in history
    pub async fn record_rip(
        &self,
        job_id: i64,
        disc_id: String,
        output_path: String,
        original_size: Option<i64>,
        final_size: Option<i64>,
        duration_seconds: Option<i64>,
    ) -> Result<(), String> {
        self.database
            .insert_rip(
                job_id,
                &disc_id,
                &output_path,
                original_size,
                final_size,
                duration_seconds,
            )
            .await
            .map_err(|e| format!("Failed to record rip: {}", e))?;

        Ok(())
    }
}
