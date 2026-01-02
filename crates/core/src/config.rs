//! Configuration management for RustRipper

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::RipperError;

/// Main configuration for RustRipper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// General settings
    pub general: GeneralConfig,
    /// MakeMKV-specific settings
    pub makemkv: MakeMKVConfig,
    /// FFmpeg transcoding settings
    pub ffmpeg: FFmpegConfig,
    /// Metadata API settings
    pub metadata: MetadataConfig,
    /// Output directory naming templates
    pub output: OutputConfig,
    /// Notification settings
    pub notifications: NotificationConfig,
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Output directory for ripped content
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,

    /// Optical disc device path
    #[serde(default = "default_disc_device")]
    pub disc_device: String,

    /// Auto-eject disc after successful rip
    #[serde(default)]
    pub auto_eject: bool,

    /// Check for duplicates before ripping
    #[serde(default = "default_bool_true")]
    pub check_duplicates: bool,

    /// Automatically start rip when disc is inserted
    #[serde(default)]
    pub auto_rip_on_insert: bool,

    /// Minimum free disk space required (in GB)
    #[serde(default = "default_min_free_gb")]
    pub min_free_space_gb: u64,
}

/// MakeMKV configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MakeMKVConfig {
    /// Path to makemkvcon executable
    #[serde(default = "default_makemkvcon_path")]
    pub executable: String,

    /// Minimum title length in seconds (skip shorter titles)
    #[serde(default = "default_min_title_length")]
    pub min_title_length: u32,

    /// Select specific titles: "all" or comma-separated numbers (e.g., "1,3,5")
    #[serde(default = "default_title_selection")]
    pub title_selection: String,

    /// Additional arguments to pass to makemkvcon
    #[serde(default)]
    pub extra_args: String,
}

/// FFmpeg transcoding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FFmpegConfig {
    /// Enable transcoding after ripping
    #[serde(default)]
    pub enabled: bool,

    /// Path to ffmpeg executable
    #[serde(default = "default_ffmpeg_path")]
    pub executable: String,

    /// Transcoding preset: "fast", "balanced", "quality", "compatible", "hardware"
    #[serde(default = "default_transcode_preset")]
    pub preset: String,

    /// Codec: "h264", "h265", "h265_nvenc", "h264_qsv", etc.
    #[serde(default = "default_video_codec")]
    pub video_codec: String,

    /// CRF (quality): 0-51 (lower = better, default 20)
    #[serde(default = "default_crf")]
    pub crf: u8,

    /// Keep original files after successful transcode
    #[serde(default)]
    pub keep_original: bool,
}

/// Metadata API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    /// TMDb API key (get from https://www.themoviedb.org/settings/api)
    pub tmdb_api_key: Option<String>,

    /// OMDb API key (get from http://www.omdbapi.com/apikey.aspx)
    pub omdb_api_key: Option<String>,

    /// TheTVDB API key (get from https://www.thetvdb.com/api-information)
    pub tvdb_api_key: Option<String>,

    /// Preferred language for metadata (ISO 639-1 code, e.g., "en", "ja")
    #[serde(default = "default_language")]
    pub preferred_language: String,

    /// Download artwork (poster images)
    #[serde(default = "default_bool_true")]
    pub download_artwork: bool,

    /// Source priority order: "tmdb,anilist,omdb,tvdb"
    #[serde(default = "default_metadata_priority")]
    pub source_priority: String,
}

/// Output directory naming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Naming template for movies: "{title} ({year})"
    #[serde(default = "default_movie_template")]
    pub movie_template: String,

    /// Naming template for TV shows: "{title}/Season {season}"
    #[serde(default = "default_tv_template")]
    pub tv_template: String,

    /// Naming template for anime: "{title} ({year})"
    #[serde(default = "default_anime_template")]
    pub anime_template: String,
}

/// Notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Send desktop notifications
    #[serde(default = "default_bool_true")]
    pub desktop_notifications: bool,

    /// Play sound alerts
    #[serde(default)]
    pub sound_alerts: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            makemkv: MakeMKVConfig::default(),
            ffmpeg: FFmpegConfig::default(),
            metadata: MetadataConfig::default(),
            output: OutputConfig::default(),
            notifications: NotificationConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            disc_device: default_disc_device(),
            auto_eject: false,
            check_duplicates: true,
            auto_rip_on_insert: false,
            min_free_space_gb: 10,
        }
    }
}

impl Default for MakeMKVConfig {
    fn default() -> Self {
        Self {
            executable: default_makemkvcon_path(),
            min_title_length: 180,
            title_selection: default_title_selection(),
            extra_args: String::new(),
        }
    }
}

impl Default for FFmpegConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            executable: default_ffmpeg_path(),
            preset: default_transcode_preset(),
            video_codec: default_video_codec(),
            crf: default_crf(),
            keep_original: false,
        }
    }
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            tmdb_api_key: None,
            omdb_api_key: None,
            tvdb_api_key: None,
            preferred_language: default_language(),
            download_artwork: true,
            source_priority: default_metadata_priority(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            movie_template: default_movie_template(),
            tv_template: default_tv_template(),
            anime_template: default_anime_template(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            desktop_notifications: true,
            sound_alerts: false,
        }
    }
}

impl Config {
    /// Get the default configuration path
    pub fn default_path() -> std::path::PathBuf {
        let config_dir = shellexpand::tilde("~/.config/masterrustripper");
        std::path::PathBuf::from(config_dir.as_ref()).join("config.toml")
    }

    /// Load configuration from default path
    pub fn load() -> crate::Result<Self> {
        Self::load_or_default(&Self::default_path())
    }

    /// Load configuration from TOML file, or return defaults if not found
    pub fn load_or_default(path: &std::path::Path) -> crate::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            toml::from_str(&content).map_err(|e| {
                RipperError::ConfigError(format!("Failed to parse config: {}", e))
            })
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to TOML file
    pub fn save(&self, path: &std::path::Path) -> crate::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> crate::Result<()> {
        // Check output directory is accessible
        if !self.general.output_dir.exists() {
            std::fs::create_dir_all(&self.general.output_dir)?;
        }

        // Check CRF value
        if self.ffmpeg.crf > 51 {
            return Err(RipperError::InvalidConfig(
                "FFmpeg CRF must be between 0 and 51".to_string(),
            ));
        }

        // Check minimum title length
        if self.makemkv.min_title_length == 0 {
            return Err(RipperError::InvalidConfig(
                "MakeMKV minimum title length must be > 0".to_string(),
            ));
        }

        Ok(())
    }
}

// Default values as functions
fn default_output_dir() -> PathBuf {
    PathBuf::from(
        shellexpand::tilde("~/Videos/Ripped")
            .as_ref()
            .to_string(),
    )
}

fn default_disc_device() -> String {
    "/dev/sr0".to_string()
}

fn default_makemkvcon_path() -> String {
    "makemkvcon".to_string()
}

fn default_min_title_length() -> u32 {
    180 // 3 minutes
}

fn default_title_selection() -> String {
    "all".to_string()
}

fn default_ffmpeg_path() -> String {
    "ffmpeg".to_string()
}

fn default_transcode_preset() -> String {
    "balanced".to_string()
}

fn default_video_codec() -> String {
    "h265".to_string()
}

fn default_crf() -> u8 {
    20
}

fn default_language() -> String {
    "en".to_string()
}

fn default_metadata_priority() -> String {
    "tmdb,anilist,omdb,tvdb".to_string()
}

fn default_movie_template() -> String {
    "{title} ({year})".to_string()
}

fn default_tv_template() -> String {
    "{title}/Season {season}".to_string()
}

fn default_anime_template() -> String {
    "{title} ({year})".to_string()
}

fn default_bool_true() -> bool {
    true
}

fn default_min_free_gb() -> u64 {
    10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.makemkv.min_title_length, 180);
        assert_eq!(config.ffmpeg.crf, 20);
        assert!(config.metadata.download_artwork);
    }

    #[test]
    fn test_config_validate_crf() {
        let mut config = Config::default();
        config.ffmpeg.crf = 52;
        assert!(config.validate().is_err());
    }
}
