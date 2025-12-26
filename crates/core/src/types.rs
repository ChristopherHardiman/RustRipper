//! Core data types for RustRipper

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Type of media on the disc
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    /// Feature film or movie
    Movie,
    /// Television series or season
    TVShow,
    /// Anime series or film
    Anime,
    /// Music concert or album
    Music,
    /// Unknown or generic media
    Unknown,
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaType::Movie => write!(f, "Movie"),
            MediaType::TVShow => write!(f, "TV Show"),
            MediaType::Anime => write!(f, "Anime"),
            MediaType::Music => write!(f, "Music"),
            MediaType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Information about a detected disc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscInfo {
    /// Device path (e.g., /dev/sr0)
    pub device: String,
    /// Raw disc label from filesystem
    pub label: String,
    /// Type of disc (DVD, Blu-ray, CD)
    pub disc_type: DiscType,
    /// True if disc is currently readable
    pub readable: bool,
}

/// Physical disc type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DiscType {
    /// Digital Versatile Disc
    DVD,
    /// Blu-ray Disc
    BluRay,
    /// Compact Disc
    CD,
    /// Ultra HD Blu-ray
    UHD,
    /// Unknown disc type
    Unknown,
}

impl std::fmt::Display for DiscType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscType::DVD => write!(f, "DVD"),
            DiscType::BluRay => write!(f, "Blu-ray"),
            DiscType::CD => write!(f, "CD"),
            DiscType::UHD => write!(f, "Ultra HD"),
            DiscType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Metadata information for media
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    /// Unique identifier (IMDb ID, TMDb ID, AniList ID, etc.)
    pub id: String,
    /// Media title
    pub title: String,
    /// Year of release
    pub year: Option<u16>,
    /// Short description or plot summary
    pub description: Option<String>,
    /// Media type (Movie, TV, Anime, etc.)
    pub media_type: MediaType,
    /// URL to poster image
    pub poster_url: Option<String>,
    /// IMDb ID if available
    pub imdb_id: Option<String>,
    /// TMDb ID if available
    pub tmdb_id: Option<String>,
    /// AniList ID if available
    pub anilist_id: Option<String>,
    /// Original source of this metadata
    pub source: String,
}

/// Information about a ripping job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    /// Unique job identifier
    pub id: Uuid,
    /// Raw disc label
    pub disc_label: String,
    /// Resolved media information
    pub media_info: Option<MediaInfo>,
    /// Current job status
    pub status: JobStatus,
    /// Current stage of the job
    pub stage: JobStage,
    /// Progress percentage (0.0-100.0)
    pub progress: f32,
    /// When the job was created
    pub created_at: DateTime<Utc>,
    /// When the job started executing
    pub started_at: Option<DateTime<Utc>>,
    /// When the job completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Output file path if completed
    pub output_path: Option<PathBuf>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Status of a ripping job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Waiting in queue
    Queued,
    /// Currently executing
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed with error
    Failed,
    /// Job was cancelled by user
    Cancelled,
    /// Job paused temporarily
    Paused,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Queued => write!(f, "Queued"),
            JobStatus::Running => write!(f, "Running"),
            JobStatus::Completed => write!(f, "Completed"),
            JobStatus::Failed => write!(f, "Failed"),
            JobStatus::Cancelled => write!(f, "Cancelled"),
            JobStatus::Paused => write!(f, "Paused"),
        }
    }
}

/// Stage of the ripping workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStage {
    /// Detecting disc
    Detecting,
    /// Fetching metadata from APIs
    FetchingMetadata,
    /// Preparing output directory
    Preparing,
    /// Running MakeMKV rip
    Ripping,
    /// Running FFmpeg transcode
    Transcoding,
    /// Finalizing and cleanup
    Finalizing,
}

impl std::fmt::Display for JobStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStage::Detecting => write!(f, "Detecting"),
            JobStage::FetchingMetadata => write!(f, "Fetching Metadata"),
            JobStage::Preparing => write!(f, "Preparing"),
            JobStage::Ripping => write!(f, "Ripping"),
            JobStage::Transcoding => write!(f, "Transcoding"),
            JobStage::Finalizing => write!(f, "Finalizing"),
        }
    }
}

/// Statistics about a completed rip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RipStats {
    /// Original file size in bytes
    pub original_size: u64,
    /// Final file size in bytes (after transcode)
    pub final_size: u64,
    /// Media duration in seconds
    pub duration_seconds: u64,
    /// Compression ratio (final / original)
    pub compression_ratio: f32,
}

impl RipStats {
    /// Calculate bytes saved
    pub fn bytes_saved(&self) -> u64 {
        self.original_size.saturating_sub(self.final_size)
    }

    /// Calculate percentage saved
    pub fn percent_saved(&self) -> f32 {
        if self.original_size == 0 {
            0.0
        } else {
            (self.bytes_saved() as f32 / self.original_size as f32) * 100.0
        }
    }
}

/// Audio track information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrack {
    /// Track index
    pub index: usize,
    /// Audio codec (aac, ac3, dts, etc.)
    pub codec: String,
    /// Language code (e.g., "eng", "jpn", "fra")
    pub language: Option<String>,
    /// Number of channels (2.0, 5.1, 7.1)
    pub channels: String,
    /// Bitrate in kbps
    pub bitrate: Option<u32>,
}

/// Subtitle track information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    /// Track index
    pub index: usize,
    /// Subtitle codec (subrip, ass, pgs, etc.)
    pub codec: String,
    /// Language code
    pub language: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_display() {
        assert_eq!(MediaType::Movie.to_string(), "Movie");
        assert_eq!(MediaType::TVShow.to_string(), "TV Show");
        assert_eq!(MediaType::Anime.to_string(), "Anime");
    }

    #[test]
    fn test_disc_type_display() {
        assert_eq!(DiscType::DVD.to_string(), "DVD");
        assert_eq!(DiscType::BluRay.to_string(), "Blu-ray");
        assert_eq!(DiscType::UHD.to_string(), "Ultra HD");
    }

    #[test]
    fn test_rip_stats_calculation() {
        let stats = RipStats {
            original_size: 10_000_000_000,
            final_size: 4_000_000_000,
            duration_seconds: 7200,
            compression_ratio: 0.4,
        };

        assert_eq!(stats.bytes_saved(), 6_000_000_000);
        assert!((stats.percent_saved() - 60.0).abs() < 0.1);
    }
}
