use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use rustripper_core::Config;
use rustripper_ripper::MakeMKVRipper;
use rustripper_transcode::{FFmpegTranscoder, TranscodePreset};

use crate::jobs::{Job, JobQueue, JobStage, JobStatus};

#[derive(Clone)]
pub struct JobExecutor {
    queue: JobQueue,
    config: Arc<RwLock<Config>>,
    is_running: Arc<RwLock<bool>>,
}

impl JobExecutor {
    pub fn new(queue: JobQueue, config: Config) -> Self {
        Self {
            queue,
            config: Arc::new(RwLock::new(config)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the job executor background task
    pub async fn start(self) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            warn!("Job executor is already running");
            return;
        }
        *is_running = true;
        drop(is_running);

        info!("Job executor started");

        tokio::spawn(async move {
            loop {
                // Check if there are jobs in the queue
                if self.queue.is_empty().await {
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }

                // Dequeue next job
                if let Some(job) = self.queue.dequeue().await {
                    if let Some(job_id) = job.id {
                        info!("Processing job {}: {}", job_id, job.disc_label);
                        
                        // Execute the job
                        if let Err(e) = self.execute_job(job).await {
                            error!("Job {} failed: {}", job_id, e);
                            let _ = self.queue.fail_job(job_id, e).await;
                        }
                    }
                }

                // Small delay between jobs
                sleep(Duration::from_millis(500)).await;
            }
        });
    }

    /// Stop the job executor
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        info!("Job executor stopped");
    }

    /// Execute a single job: rip → transcode → finalize
    async fn execute_job(&self, mut job: Job) -> Result<(), String> {
        let job_id = job.id.ok_or("Job has no ID")?;
        let config = self.config.read().await;

        // Stage 1: Ripping
        info!("Job {}: Starting rip stage", job_id);
        let _ = self
            .queue
            .update_job_status(job_id, JobStatus::Ripping, 0.0, Some(JobStage::Ripping))
            .await;

        let output_dir = config.general.output_dir.clone();
        let rip_output = self.execute_rip(&job, &config, job_id).await?;

        // Stage 2: Transcoding (if enabled)
        let final_output = if config.ffmpeg.enabled {
            info!("Job {}: Starting transcode stage", job_id);
            let _ = self
                .queue
                .update_job_status(
                    job_id,
                    JobStatus::Transcoding,
                    50.0,
                    Some(JobStage::Transcoding),
                )
                .await;

            self.execute_transcode(&rip_output, &config, job_id).await?
        } else {
            rip_output
        };

        // Stage 3: Finalize
        info!("Job {}: Finalizing", job_id);
        let _ = self
            .queue
            .update_job_status(job_id, JobStatus::Completed, 95.0, Some(JobStage::Finalizing))
            .await;

        // Calculate file sizes
        let original_size = std::fs::metadata(&rip_output)
            .ok()
            .map(|m| m.len() as i64);
        let final_size = std::fs::metadata(&final_output)
            .ok()
            .map(|m| m.len() as i64);

        // Generate disc ID for duplicate detection
        let disc_id = self.generate_disc_id(&job.disc_label);

        // Record in history
        let _ = self
            .queue
            .record_rip(
                job_id,
                disc_id,
                final_output.to_string_lossy().to_string(),
                original_size,
                final_size,
                None,
            )
            .await;

        // Delete original if configured
        if config.ffmpeg.enabled && !config.ffmpeg.keep_original {
            if let Err(e) = std::fs::remove_file(&rip_output) {
                warn!("Failed to delete original file: {}", e);
            } else {
                debug!("Deleted original file: {:?}", rip_output);
            }
        }

        // Complete the job
        let _ = self
            .queue
            .complete_job(job_id, final_output.to_string_lossy().to_string())
            .await;

        Ok(())
    }

    /// Execute MakeMKV rip
    async fn execute_rip(
        &self,
        job: &Job,
        config: &Config,
        job_id: i64,
    ) -> Result<PathBuf, String> {
        let ripper = MakeMKVRipper::new(&config.makemkv.executable)
            .map_err(|e| format!("Failed to create MakeMKV ripper: {}", e))?;

        let output_dir = config.general.output_dir.clone();
        
        // Create output filename
        let filename = if let Some(title) = &job.title {
            if let Some(year) = job.year {
                format!("{} ({})", title, year)
            } else {
                title.clone()
            }
        } else {
            job.disc_label.clone()
        };

        let output_path = output_dir.join(format!("{}.mkv", filename));

        // Execute rip with progress callback
        let queue = self.queue.clone();
        let progress_callback = move |progress: f64| {
            let queue = queue.clone();
            tokio::spawn(async move {
                let _ = queue
                    .update_job_status(job_id, JobStatus::Ripping, progress, Some(JobStage::Ripping))
                    .await;
            });
        };

        ripper
            .rip(
                &config.general.disc_device,
                &output_path,
                config.makemkv.min_title_length,
                Some(progress_callback),
            )
            .await
            .map_err(|e| format!("MakeMKV rip failed: {}", e))?;

        Ok(output_path)
    }

    /// Execute FFmpeg transcode
    async fn execute_transcode(
        &self,
        input_path: &PathBuf,
        config: &Config,
        job_id: i64,
    ) -> Result<PathBuf, String> {
        let transcoder = FFmpegTranscoder::new(&config.ffmpeg.executable)
            .map_err(|e| format!("Failed to create transcoder: {}", e))?;

        // Determine output path
        let output_path = input_path.with_extension("transcoded.mkv");

        // Parse preset from config
        let preset = config.ffmpeg.preset.parse::<TranscodePreset>()
            .unwrap_or(TranscodePreset::Balanced);

        // Execute transcode with progress callback
        let queue = self.queue.clone();
        let progress_callback = move |progress: f64| {
            let queue = queue.clone();
            tokio::spawn(async move {
                let adjusted_progress = 50.0 + (progress * 0.45); // 50-95%
                let _ = queue
                    .update_job_status(
                        job_id,
                        JobStatus::Transcoding,
                        adjusted_progress,
                        Some(JobStage::Transcoding),
                    )
                    .await;
            });
        };

        transcoder
            .transcode(input_path, &output_path, preset, Some(progress_callback))
            .await
            .map_err(|e| format!("Transcode failed: {}", e))?;

        Ok(output_path)
    }

    /// Generate a unique disc ID for duplicate detection
    fn generate_disc_id(&self, disc_label: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(disc_label.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
