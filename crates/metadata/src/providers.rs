//! Metadata provider trait and implementations

use rustripper_core::{MediaInfo, Result};

/// Types of metadata providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// The Movie Database (TMDb)
    TMDb,
    /// AniList (Anime)
    AniList,
    /// Open Movie Database (OMDb)
    OMDb,
    /// TheTVDB
    TheTVDB,
}

/// Trait for metadata providers
pub trait MetadataProvider: Send + Sync {
    /// Search for media by query string
    fn search(&self, query: &str) -> Result<Vec<MediaInfo>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type() {
        assert_eq!(ProviderType::TMDb, ProviderType::TMDb);
    }
}
