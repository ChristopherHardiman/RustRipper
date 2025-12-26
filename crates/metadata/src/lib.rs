//! Metadata lookup and aggregation from multiple providers (TMDb, AniList, OMDb, TheTVDB)

pub mod providers;
pub mod aggregator;
pub mod omdb;
pub mod tmdb;
pub mod anilist;

pub use aggregator::MetadataAggregator;
pub use providers::{MetadataProvider, ProviderType};
pub use omdb::OmdbProvider;
pub use tmdb::TmdbProvider;
pub use anilist::AnilistProvider;
