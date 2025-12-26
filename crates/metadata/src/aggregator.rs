//! Metadata aggregator that queries multiple providers in priority order

use rustripper_core::{MediaInfo, MediaType, Result, RipperError};
use crate::{TmdbProvider, AnilistProvider, OmdbProvider};

/// Aggregates metadata from multiple providers
pub struct MetadataAggregator {
    tmdb: Option<TmdbProvider>,
    anilist: AnilistProvider,
    omdb: Option<OmdbProvider>,
}

impl MetadataAggregator {
    /// Create a new metadata aggregator
    pub fn new() -> Self {
        Self {
            tmdb: None,
            anilist: AnilistProvider::new(),
            omdb: None,
        }
    }

    /// Add TMDb provider with API key
    pub fn with_tmdb(mut self, api_key: impl Into<String>) -> Self {
        self.tmdb = Some(TmdbProvider::new(api_key));
        self
    }

    /// Add OMDb provider with API key
    pub fn with_omdb(mut self, api_key: impl Into<String>) -> Self {
        self.omdb = Some(OmdbProvider::new(api_key));
        self
    }

    /// Search across all providers in priority order
    /// Priority: TMDb → AniList → OMDb
    pub async fn search(&self, query: &str, year: Option<u16>) -> Result<Vec<MediaInfo>> {
        // Sanitize the disc label first
        let (sanitized_query, extracted_year) = sanitize_disc_label(query);
        let search_year = year.or(extracted_year);

        let mut all_results = Vec::new();
        let mut errors = Vec::new();

        // Try TMDb first (best for movies and TV)
        if let Some(tmdb) = &self.tmdb {
            match tmdb.search(&sanitized_query, search_year).await {
                Ok(results) => all_results.extend(results),
                Err(e) => errors.push(format!("TMDb: {}", e)),
            }
        }

        // Try AniList (best for anime, no API key needed)
        match self.anilist.search(&sanitized_query, search_year).await {
            Ok(results) => all_results.extend(results),
            Err(e) => errors.push(format!("AniList: {}", e)),
        }

        // Try OMDb as fallback
        if let Some(omdb) = &self.omdb {
            match omdb.search(&sanitized_query).await {
                Ok(results) => all_results.extend(results),
                Err(e) => errors.push(format!("OMDb: {}", e)),
            }
        }

        if all_results.is_empty() {
            return Err(RipperError::AllProvidersFailedError(format!(
                "Query: '{}'. Errors: {}",
                sanitized_query,
                errors.join("; ")
            )));
        }

        // Deduplicate by title and year
        all_results = deduplicate_results(all_results);

        Ok(all_results)
    }

    /// Search with automatic media type detection
    pub async fn search_with_type_detection(&self, disc_label: &str) -> Result<Vec<MediaInfo>> {
        let (sanitized, year) = sanitize_disc_label(disc_label);
        let media_type = detect_media_type(&sanitized);

        let mut results = self.search(&sanitized, year).await?;

        // Filter results by detected media type if confident
        if let Some(detected_type) = media_type {
            results = results
                .into_iter()
                .filter(|r| r.media_type == detected_type)
                .collect();
        }

        Ok(results)
    }

    /// Get the best match (first result - providers are in priority order)
    pub async fn get_best_match(&self, query: &str, year: Option<u16>) -> Result<MediaInfo> {
        let results = self.search(query, year).await?;

        results
            .into_iter()
            .next()
            .ok_or_else(|| RipperError::metadata_error(query, "No results found"))
    }
}

impl Default for MetadataAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitize disc labels to improve search accuracy
/// - Remove underscores, replace with spaces
/// - Extract year from formats like "Title_2010" or "Title (2010)"
/// - Remove common disc identifiers (DISC1, D1, etc.)
pub fn sanitize_disc_label(label: &str) -> (String, Option<u16>) {
    let mut sanitized = label.to_string();
    let mut year = None;

    // First, remove disc identifiers so year can be detected
    sanitized = regex::Regex::new(r"(?i)[_\s]*(disc|disk|d|cd)\s*\d+[_\s]*$")
        .ok()
        .map(|re| re.replace(&sanitized, "").to_string())
        .unwrap_or(sanitized);

    // Extract year from patterns like (2010) or _2010 at the end
    if let Some(captures) = regex::Regex::new(r"[_\s\(](\d{4})[\)\s_]*$")
        .ok()
        .and_then(|re| re.captures(&sanitized))
    {
        if let Some(year_str) = captures.get(1) {
            if let Ok(y) = year_str.as_str().parse::<u16>() {
                if (1900..=2100).contains(&y) {
                    year = Some(y);
                    // Remove the year from the title
                    sanitized = sanitized[..captures.get(0).unwrap().start()].to_string();
                }
            }
        }
    }

    // Replace underscores with spaces
    sanitized = sanitized.replace('_', " ");

    // Clean up extra whitespace
    sanitized = sanitized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    (sanitized, year)
}

/// Detect media type from title keywords
fn detect_media_type(title: &str) -> Option<MediaType> {
    let lower = title.to_lowercase();

    // Anime indicators
    if lower.contains("season")
        || lower.contains("episode")
        || lower.contains("vol")
        || lower.contains("op ")
        || lower.contains("ed ")
    {
        return Some(MediaType::Anime);
    }

    // TV show indicators
    if lower.contains("s01")
        || lower.contains("s02")
        || lower.contains(" s1")
        || lower.contains(" s2")
    {
        return Some(MediaType::TVShow);
    }

    None // Can't determine automatically
}

/// Remove duplicate results based on title and year similarity
fn deduplicate_results(results: Vec<MediaInfo>) -> Vec<MediaInfo> {
    let mut unique = Vec::new();

    for result in results {
        let is_duplicate = unique.iter().any(|existing: &MediaInfo| {
            // Consider duplicates if titles match (case-insensitive) and years match
            existing.title.eq_ignore_ascii_case(&result.title)
                && existing.year == result.year
        });

        if !is_duplicate {
            unique.push(result);
        }
    }

    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregator_creation() {
        let agg = MetadataAggregator::new();
        assert!(agg.tmdb.is_none());
        assert!(agg.omdb.is_none());
    }

    #[test]
    fn test_with_tmdb() {
        let agg = MetadataAggregator::new().with_tmdb("test_key");
        assert!(agg.tmdb.is_some());
    }

    #[test]
    fn test_with_omdb() {
        let agg = MetadataAggregator::new().with_omdb("test_key");
        assert!(agg.omdb.is_some());
    }

    #[test]
    fn test_sanitize_disc_label_underscores() {
        let (sanitized, year) = sanitize_disc_label("The_Matrix_1999");
        assert_eq!(sanitized, "The Matrix");
        assert_eq!(year, Some(1999));
    }

    #[test]
    fn test_sanitize_disc_label_parentheses() {
        let (sanitized, year) = sanitize_disc_label("Inception (2010)");
        assert_eq!(sanitized, "Inception");
        assert_eq!(year, Some(2010));
    }

    #[test]
    fn test_sanitize_disc_label_disc_number() {
        let (sanitized, year) = sanitize_disc_label("Breaking_Bad_Disc1");
        assert_eq!(sanitized, "Breaking Bad");
        assert_eq!(year, None);
    }

    #[test]
    fn test_sanitize_disc_label_complex() {
        let (sanitized, year) = sanitize_disc_label("Cowboy_Bebop_1998_D1");
        assert_eq!(sanitized, "Cowboy Bebop");
        assert_eq!(year, Some(1998));
    }

    #[test]
    fn test_sanitize_disc_label_no_year() {
        let (sanitized, year) = sanitize_disc_label("Some_Movie_Title");
        assert_eq!(sanitized, "Some Movie Title");
        assert_eq!(year, None);
    }

    #[test]
    fn test_sanitize_disc_label_whitespace() {
        let (sanitized, _) = sanitize_disc_label("Title   With   Extra   Spaces");
        assert_eq!(sanitized, "Title With Extra Spaces");
    }

    #[test]
    fn test_detect_media_type_anime() {
        assert_eq!(
            detect_media_type("Attack on Titan Season 1"),
            Some(MediaType::Anime)
        );
        assert_eq!(
            detect_media_type("Cowboy Bebop Vol 1"),
            Some(MediaType::Anime)
        );
    }

    #[test]
    fn test_detect_media_type_tv() {
        assert_eq!(
            detect_media_type("Breaking Bad S01"),
            Some(MediaType::TVShow)
        );
        assert_eq!(
            detect_media_type("Game of Thrones s1"),
            Some(MediaType::TVShow)
        );
    }

    #[test]
    fn test_detect_media_type_unknown() {
        assert_eq!(detect_media_type("The Matrix"), None);
        assert_eq!(detect_media_type("Inception"), None);
    }

    #[test]
    fn test_deduplicate_results() {
        let results = vec![
            MediaInfo {
                id: "tmdb:123".to_string(),
                title: "The Matrix".to_string(),
                year: Some(1999),
                description: None,
                media_type: MediaType::Movie,
                poster_url: None,
                imdb_id: None,
                tmdb_id: Some("123".to_string()),
                anilist_id: None,
                source: "TMDb".to_string(),
            },
            MediaInfo {
                id: "omdb:tt0133093".to_string(),
                title: "The Matrix".to_string(),
                year: Some(1999),
                description: Some("Different source".to_string()),
                media_type: MediaType::Movie,
                poster_url: None,
                imdb_id: Some("tt0133093".to_string()),
                tmdb_id: None,
                anilist_id: None,
                source: "OMDb".to_string(),
            },
            MediaInfo {
                id: "tmdb:124".to_string(),
                title: "The Matrix Reloaded".to_string(),
                year: Some(2003),
                description: None,
                media_type: MediaType::Movie,
                poster_url: None,
                imdb_id: None,
                tmdb_id: Some("124".to_string()),
                anilist_id: None,
                source: "TMDb".to_string(),
            },
        ];

        let unique = deduplicate_results(results);
        assert_eq!(unique.len(), 2);
        assert_eq!(unique[0].title, "The Matrix");
        assert_eq!(unique[1].title, "The Matrix Reloaded");
    }
}
