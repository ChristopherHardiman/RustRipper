use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use rustripper_core::Config;
use rustripper_disc::DiscWatcher as CoreDiscWatcher;
use rustripper_metadata::MetadataAggregator;

use crate::api::WebSocketEvent;
use crate::jobs::{Job, JobQueue};

#[derive(Clone)]
pub struct DiscWatcher {
    config: Arc<RwLock<Config>>,
    job_queue: JobQueue,
    disc_present: Arc<RwLock<bool>>,
    current_disc_label: Arc<RwLock<Option<String>>>,
    last_disc_label: Arc<RwLock<Option<String>>>,
}

impl DiscWatcher {
    pub fn new(
        config: Arc<RwLock<Config>>,
        job_queue: JobQueue,
        disc_present: Arc<RwLock<bool>>,
        current_disc_label: Arc<RwLock<Option<String>>>,
    ) -> Self {
        Self {
            config,
            job_queue,
            disc_present,
            current_disc_label,
            last_disc_label: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the disc watcher background task
    pub async fn start(self) {
        info!("Disc watcher started");

        tokio::spawn(async move {
            loop {
                if let Err(e) = self.check_disc().await {
                    error!("Disc watcher error: {}", e);
                }

                // Poll every 2 seconds
                sleep(Duration::from_secs(2)).await;
            }
        });
    }

    /// Check for disc changes
    async fn check_disc(&self) -> Result<(), String> {
        let config = self.config.read().await;
        
        // Create disc watcher with device from config
        let watcher = CoreDiscWatcher::new(&config.general.disc_device, Duration::from_secs(2))
            .map_err(|e| format!("Failed to create disc watcher: {}", e))?;

        // Detect disc
        match watcher.detect_disc().await {
            Ok(Some(disc_info)) => {
                let label = disc_info.label.clone();
                let disc_type = format!("{:?}", disc_info.disc_type);

                // Check if this is a new disc
                let last_label = self.last_disc_label.read().await.clone();
                let is_new_disc = last_label.as_ref() != Some(&label);

                if is_new_disc {
                    info!("New disc detected: {} ({})", label, disc_type);

                    // Update state
                    *self.disc_present.write().await = true;
                    *self.current_disc_label.write().await = Some(label.clone());
                    *self.last_disc_label.write().await = Some(label.clone());

                    // Emit WebSocket event
                    let event = WebSocketEvent::DiscInserted {
                        disc_label: label.clone(),
                        disc_type: disc_type.clone(),
                    };
                    let _ = self.job_queue.queue.clone(); // Access to emit event via queue's event sender

                    // Handle disc insertion
                    if config.general.auto_rip_on_insert {
                        self.handle_disc_insertion(label, &config).await?;
                    }
                }
            }
            Ok(None) => {
                // No disc present
                let was_present = *self.disc_present.read().await;
                
                if was_present {
                    info!("Disc ejected");
                    
                    // Update state
                    *self.disc_present.write().await = false;
                    *self.current_disc_label.write().await = None;
                    *self.last_disc_label.write().await = None;

                    // Emit WebSocket event (would need access to ws_state)
                    // let _ = self.ws_state.send(WebSocketEvent::DiscEjected);
                }
            }
            Err(e) => {
                debug!("Disc detection error (this is normal if no disc): {}", e);
            }
        }

        Ok(())
    }

    /// Handle disc insertion: fetch metadata and create job
    async fn handle_disc_insertion(
        &self,
        disc_label: String,
        config: &Config,
    ) -> Result<(), String> {
        info!("Handling disc insertion: {}", disc_label);

        // Generate disc ID for duplicate detection
        let disc_id = self.generate_disc_id(&disc_label);

        // Check for duplicates
        if config.general.check_duplicates {
            match self.job_queue.is_duplicate(&disc_id).await {
                Ok(true) => {
                    warn!("Disc '{}' has already been ripped (disc_id: {})", disc_label, disc_id);
                    info!("Skipping duplicate disc");
                    return Ok(());
                }
                }
                Ok(false) => {
                    debug!("Disc '{}' is not a duplicate", disc_label);
                }
                Err(e) => {
                    warn!("Failed to check for duplicate: {}", e);
                }
            }
        }

        // Fetch metadata
        let (title, year) = self.fetch_metadata(&disc_label, config).await;

        // Create and enqueue job
        let mut job = Job::new(disc_label.clone());
        if let Some(title_str) = title {
            job = job.with_metadata(title_str, year);
        }

        match self.job_queue.enqueue(job).await {
            Ok(job_id) => {
                info!("Auto-created job {} for disc: {}", job_id, disc_label);
                Ok(())
            }
            Err(e) => {
                error!("Failed to create job for disc: {}", e);
                Err(e)
            }
        }
    }

    /// Fetch metadata from configured providers
    async fn fetch_metadata(
        &self,
        disc_label: &str,
        config: &Config,
    ) -> (Option<String>, Option<i32>) {
        // Create metadata aggregator
        let mut aggregator = MetadataAggregator::new();

        if let Some(api_key) = &config.metadata_tmdb_api_key {
            if !api_key.is_empty() {.tmdb_api_key {
            if !api_key.is_empty() {
                aggregator = aggregator.with_tmdb(api_key);
            }
        }

        if let Some(api_key) = &config.metadata.
                aggregator = aggregator.with_omdb(api_key);
            }
        }

        // Use aggregator to search for disc
        match aggregator.search_disc(disc_label).await {
            Ok(Some(metadata)) => {
                info!(
                    "Found metadata: {} ({})",
                    metadata.title,
                    metadata.year.unwrap_or(0)
                );
                (Some(metadata.title), metadata.year)
            }
            Ok(None) => {
                warn!("No metadata found for disc: {}", disc_label);
                (None, None)
            }
            Err(e) => {
                warn!("Failed to fetch metadata: {}", e);
                (None, None)
            }
        }
    }

    /// Generate a unique disc ID for duplicate detection
    fn generate_disc_id(&self, disc_label: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(disc_label.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
