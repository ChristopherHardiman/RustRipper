//! AniList GraphQL API client for anime metadata

use rustripper_core::{MediaInfo, MediaType, Result, RipperError};
use serde::{Deserialize, Serialize};

/// AniList API client (GraphQL)
pub struct AnilistProvider {
    client: reqwest::Client,
    api_url: String,
}

impl AnilistProvider {
    /// Create a new AniList provider (no API key required)
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url: "https://graphql.anilist.co".to_string(),
        }
    }

    /// Search for anime by title
    pub async fn search(&self, query: &str, year: Option<u16>) -> Result<Vec<MediaInfo>> {
        let graphql_query = build_search_query(query, year);

        let response = self
            .client
            .post(&self.api_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&graphql_query)
            .send()
            .await
            .map_err(|e| RipperError::network_error(format!("AniList request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RipperError::metadata_error(
                query,
                format!("AniList API error: {}", response.status()),
            ));
        }

        let anilist_response: AnilistResponse = response.json().await.map_err(|e| {
            RipperError::metadata_error(query, format!("Failed to parse AniList response: {}", e))
        })?;

        if let Some(data) = anilist_response.data {
            if let Some(page) = data.page {
                if !page.media.is_empty() {
                    return Ok(page
                        .media
                        .into_iter()
                        .map(|anime| anime.to_media_info())
                        .collect());
                }
            }
        }

        Err(RipperError::metadata_error(
            query,
            "No results found on AniList",
        ))
    }

    /// Get anime details by ID
    pub async fn get_anime(&self, anime_id: u32) -> Result<MediaInfo> {
        let graphql_query = build_get_query(anime_id);

        let response = self
            .client
            .post(&self.api_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&graphql_query)
            .send()
            .await
            .map_err(|e| RipperError::network_error(format!("AniList request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RipperError::MetadataError {
                query: anime_id.to_string(),
                reason: format!("AniList API error: {}", response.status()),
            });
        }

        let anilist_response: AnilistSingleResponse = response.json().await.map_err(|e| {
            RipperError::MetadataError {
                query: anime_id.to_string(),
                reason: format!("Failed to parse AniList response: {}", e),
            }
        })?;

        if let Some(data) = anilist_response.data {
            if let Some(media) = data.media {
                return Ok(media.to_media_info());
            }
        }

        Err(RipperError::MetadataError {
            query: anime_id.to_string(),
            reason: "Anime not found".to_string(),
        })
    }
}

impl Default for AnilistProvider {
    fn default() -> Self {
        Self::new()
    }
}

// GraphQL query builders

fn build_search_query(search_term: &str, year: Option<u16>) -> serde_json::Value {
    let query = r#"
        query ($search: String, $year: Int) {
            Page(page: 1, perPage: 10) {
                media(search: $search, seasonYear: $year, type: ANIME) {
                    id
                    title {
                        romaji
                        english
                        native
                    }
                    seasonYear
                    description
                    coverImage {
                        large
                        medium
                    }
                    bannerImage
                    averageScore
                    genres
                    format
                    episodes
                }
            }
        }
    "#;

    let mut variables = serde_json::json!({
        "search": search_term
    });

    if let Some(y) = year {
        variables["year"] = serde_json::json!(y);
    }

    serde_json::json!({
        "query": query,
        "variables": variables
    })
}

fn build_get_query(anime_id: u32) -> serde_json::Value {
    let query = r#"
        query ($id: Int) {
            Media(id: $id, type: ANIME) {
                id
                title {
                    romaji
                    english
                    native
                }
                seasonYear
                description
                coverImage {
                    large
                    medium
                }
                bannerImage
                averageScore
                genres
                format
                episodes
            }
        }
    "#;

    serde_json::json!({
        "query": query,
        "variables": {
            "id": anime_id
        }
    })
}

// AniList API response structures

#[derive(Debug, Deserialize)]
struct AnilistResponse {
    data: Option<AnilistData>,
}

#[derive(Debug, Deserialize)]
struct AnilistSingleResponse {
    data: Option<AnilistSingleData>,
}

#[derive(Debug, Deserialize)]
struct AnilistData {
    #[serde(rename = "Page")]
    page: Option<AnilistPage>,
}

#[derive(Debug, Deserialize)]
struct AnilistSingleData {
    #[serde(rename = "Media")]
    media: Option<AnilistMedia>,
}

#[derive(Debug, Deserialize)]
struct AnilistPage {
    media: Vec<AnilistMedia>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AnilistMedia {
    id: u32,
    title: AnilistTitle,
    #[serde(rename = "seasonYear")]
    season_year: Option<u16>,
    description: Option<String>,
    #[serde(rename = "coverImage")]
    cover_image: Option<AnilistImage>,
    #[serde(rename = "bannerImage")]
    banner_image: Option<String>,
    #[serde(rename = "averageScore")]
    average_score: Option<f32>,
    genres: Option<Vec<String>>,
    format: Option<String>,
    episodes: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AnilistTitle {
    romaji: Option<String>,
    english: Option<String>,
    native: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AnilistImage {
    large: Option<String>,
    medium: Option<String>,
}

impl AnilistMedia {
    fn to_media_info(self) -> MediaInfo {
        // Prefer English title, fall back to Romaji
        let title = self
            .title
            .english
            .or(self.title.romaji)
            .or(self.title.native)
            .unwrap_or_else(|| "Unknown".to_string());

        // Use cover image (large preferred)
        let poster_url = self
            .cover_image
            .and_then(|img| img.large.or(img.medium));

        // Determine media type from format
        let media_type = match self.format.as_deref() {
            Some("TV") | Some("TV_SHORT") | Some("ONA") => MediaType::TVShow,
            Some("MOVIE") => MediaType::Movie,
            _ => MediaType::Anime, // Default to Anime for other formats
        };

        // Strip HTML tags from description
        let description = self.description.map(|desc| strip_html_tags(&desc));

        MediaInfo {
            id: format!("anilist:{}", self.id),
            title,
            year: self.season_year,
            description,
            media_type,
            poster_url,
            imdb_id: None,
            tmdb_id: None,
            anilist_id: Some(self.id.to_string()),
            source: "AniList".to_string(),
        }
    }
}

/// Strip HTML tags from AniList descriptions
fn strip_html_tags(html: &str) -> String {
    // Simple HTML tag removal (for basic cases)
    let mut result = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anilist_provider_creation() {
        let provider = AnilistProvider::new();
        assert_eq!(provider.api_url, "https://graphql.anilist.co");
    }

    #[test]
    fn test_anilist_default() {
        let provider = AnilistProvider::default();
        assert_eq!(provider.api_url, "https://graphql.anilist.co");
    }

    #[test]
    fn test_anime_to_media_info() {
        let anime = AnilistMedia {
            id: 16498,
            title: AnilistTitle {
                romaji: Some("Shingeki no Kyojin".to_string()),
                english: Some("Attack on Titan".to_string()),
                native: Some("進撃の巨人".to_string()),
            },
            season_year: Some(2013),
            description: Some("<p>Test description</p>".to_string()),
            cover_image: Some(AnilistImage {
                large: Some("https://s4.anilist.co/file/anilistcdn/media/anime/cover/large/bx16498-C6FPmWm59CyP.jpg".to_string()),
                medium: None,
            }),
            banner_image: None,
            average_score: Some(84.0),
            genres: Some(vec!["Action".to_string(), "Drama".to_string()]),
            format: Some("TV".to_string()),
            episodes: Some(25),
        };

        let media_info = anime.to_media_info();
        assert_eq!(media_info.title, "Attack on Titan");
        assert_eq!(media_info.year, Some(2013));
        assert_eq!(media_info.media_type, MediaType::TVShow);
        assert!(media_info.poster_url.is_some());
        assert_eq!(media_info.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_title_fallback() {
        let anime = AnilistMedia {
            id: 1,
            title: AnilistTitle {
                romaji: Some("Romaji Title".to_string()),
                english: None,
                native: None,
            },
            season_year: None,
            description: None,
            cover_image: None,
            banner_image: None,
            average_score: None,
            genres: None,
            format: None,
            episodes: None,
        };

        let media_info = anime.to_media_info();
        assert_eq!(media_info.title, "Romaji Title");
    }

    #[test]
    fn test_media_type_from_format() {
        let movie = AnilistMedia {
            id: 1,
            title: AnilistTitle {
                romaji: Some("Test".to_string()),
                english: None,
                native: None,
            },
            season_year: None,
            description: None,
            cover_image: None,
            banner_image: None,
            average_score: None,
            genres: None,
            format: Some("MOVIE".to_string()),
            episodes: None,
        };

        let media_info = movie.to_media_info();
        assert_eq!(media_info.media_type, MediaType::Movie);
    }

    #[test]
    fn test_strip_html_tags() {
        assert_eq!(strip_html_tags("<p>Test</p>"), "Test");
        assert_eq!(strip_html_tags("<br>Line<br>Break"), "LineBreak");
        assert_eq!(
            strip_html_tags("<b>Bold</b> and <i>italic</i>"),
            "Bold and italic"
        );
        assert_eq!(strip_html_tags("No tags"), "No tags");
    }

    #[test]
    fn test_score_conversion() {
        let anime = AnilistMedia {
            id: 1,
            title: AnilistTitle {
                romaji: Some("Test".to_string()),
                english: None,
                native: None,
            },
            season_year: Some(2020),
            description: None,
            cover_image: None,
            banner_image: None,
            average_score: Some(85.0), // 85/100 should convert to 8.5/10
            genres: None,
            format: None,
            episodes: None,
        };

        let media_info = anime.to_media_info();
        assert_eq!(media_info.title, "Test");
        assert_eq!(media_info.year, Some(2020));
    }

    #[test]
    fn test_build_search_query() {
        let query = build_search_query("Cowboy Bebop", Some(1998));
        assert!(query["query"].as_str().unwrap().contains("Page"));
        assert_eq!(query["variables"]["search"], "Cowboy Bebop");
        assert_eq!(query["variables"]["year"], 1998);
    }

    #[test]
    fn test_build_search_query_no_year() {
        let query = build_search_query("Test Anime", None);
        assert!(query["variables"]["search"].as_str().is_some());
        assert!(query["variables"]["year"].is_null());
    }

    #[test]
    fn test_build_get_query() {
        let query = build_get_query(16498);
        assert!(query["query"].as_str().unwrap().contains("Media"));
        assert_eq!(query["variables"]["id"], 16498);
    }
}
