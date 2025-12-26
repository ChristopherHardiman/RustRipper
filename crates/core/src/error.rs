//! Error types for RustRipper using thiserror

use thiserror::Error;

/// Result type for RustRipper operations
pub type Result<T> = std::result::Result<T, RipperError>;

/// Comprehensive error type for all RustRipper operations
#[derive(Debug, Error)]
pub enum RipperError {
    #[error("Disc not found at {0}")]
    DiscNotFound(String),

    #[error("Failed to detect disc: {0}")]
    DiscDetectionFailed(String),

    #[error("Decryption failed: {0}. Is {1} installed?")]
    DecryptionFailed(String, String),

    #[error("KEYDB.cfg not found or outdated. Download from MakeMKV forum.")]
    KeyDbMissing,

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid configuration value: {0}")]
    InvalidConfig(String),

    #[error("MakeMKV error: {0}")]
    MakeMKVError(String),

    #[error("MakeMKV not found or not executable. Install MakeMKV first.")]
    MakeMKVNotFound,

    #[error("FFmpeg error: {0}")]
    FFmpegError(String),

    #[error("FFmpeg not found or not executable. Install FFmpeg first.")]
    FFmpegNotFound,

    #[error("Metadata lookup failed for '{query}': {reason}")]
    MetadataError { query: String, reason: String },

    #[error("All metadata providers failed for query: {0}")]
    AllProvidersFailedError(String),

    #[error("Hardware encoder {0} not available")]
    HardwareEncoderUnavailable(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Disc already in history: {id}. Set skip_duplicate=false to override.")]
    DuplicateDisc { id: String },

    #[error("Output directory not accessible: {0}")]
    OutputDirError(String),

    #[error("Insufficient disk space. Need {required} GB, have {available} GB")]
    InsufficientDiskSpace { required: u64, available: u64 },

    #[error("Job was cancelled by user")]
    JobCancelled,

    #[error("Command execution failed: {0}")]
    CommandExecutionFailed(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl RipperError {
    /// Create a metadata error with context
    pub fn metadata_error(query: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::MetadataError {
            query: query.into(),
            reason: reason.into(),
        }
    }

    /// Create a network error
    pub fn network_error(msg: impl Into<String>) -> Self {
        Self::NetworkError(msg.into())
    }

    /// Create a database error
    pub fn database_error(msg: impl Into<String>) -> Self {
        Self::DatabaseError(msg.into())
    }
}
