//! OMDb API provider for movie/TV metadata

use rustripper_core::{MediaInfo, MediaType, Result, RipperError};
use serde::Deserialize;

/// OMDb API response structure
/// Field names use PascalCase as returned by the API
#[derive(Debug, Deserialize)]
pub struct OmdbResponse {
    #[serde(rename = "Title")]
    pub title: Option<String>,
    
    #[serde(rename = "Year")]
    pub year: Option<String>,
    
    #[serde(rename = "Type")]
    pub media_type: Option<String>,
    
    #[serde(rename = "Poster")]
    pub poster: Option<String>,
    
    #[serde(rename = "Plot")]
    pub plot: Option<String>,
    
    #[serde(rename = "imdbID")]
    pub imdb_id: Option<String>,
    
    #[serde(rename = "Response")]
    pub response: String,
    
    #[serde(rename = "Error")]
    pub error: Option<String>,
}

/// OMDb API client
pub struct OmdbProvider {
    api_key: String,
    base_url: String,
}

impl OmdbProvider {
    /// Create a new OMDb provider with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "http://www.omdbapi.com/".to_string(),
        }
    }

    /// Search for media by title
    pub async fn search(&self, title: &str) -> Result<Vec<MediaInfo>> {
        let encoded_title = urlencoding::encode(title);
        let url = format!(
            "{}?apikey={}&t={}",
            self.base_url, self.api_key, encoded_title
        );

        let response = reqwest::get(&url)
            .await
            .map_err(|e| RipperError::network_error(e.to_string()))?
            .json::<OmdbResponse>()
            .await
            .map_err(|e| RipperError::network_error(e.to_string()))?;

        if response.response == "False" {
            let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
            return Err(RipperError::metadata_error(title, error_msg));
        }

        let media_info = self.parse_response(response)?;
        Ok(vec![media_info])
    }

    /// Parse OMDb response into MediaInfo
    fn parse_response(&self, response: OmdbResponse) -> Result<MediaInfo> {
        // Check for error response
        if response.response == "False" {
            return Err(RipperError::MetadataError {
                query: "OMDb".to_string(),
                reason: response.error.unwrap_or_else(|| "Unknown OMDb error".to_string()),
            });
        }

        let title = response.title
            .ok_or_else(|| RipperError::MetadataError {
                query: "OMDb".to_string(),
                reason: "Missing title in response".to_string(),
            })?;

        let year = response.year
            .as_ref()
            .and_then(|y| y.split('–').next())
            .and_then(|y| y.parse::<u16>().ok());

        let media_type = response.media_type
            .as_ref()
            .map(|t| match t.to_lowercase().as_str() {
                "movie" => MediaType::Movie,
                "series" => MediaType::TVShow,
                _ => MediaType::Unknown,
            })
            .unwrap_or(MediaType::Unknown);

        let poster_url = response.poster
            .filter(|p| p != "N/A")
            .map(|p| p.to_string());

        Ok(MediaInfo {
            id: response.imdb_id.clone().unwrap_or_default(),
            title,
            year,
            description: response.plot.filter(|p| p != "N/A"),
            media_type,
            poster_url,
            imdb_id: response.imdb_id,
            tmdb_id: None,
            anilist_id: None,
            source: "OMDb".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_omdb_response_parsing() {
        let json = r#"{
            "Title": "Inception",
            "Year": "2010",
            "Type": "movie",
            "Poster": "https://example.com/poster.jpg",
            "Plot": "A thief who steals corporate secrets.",
            "imdbID": "tt1375666",
            "Response": "True"
        }"#;

        let response: OmdbResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.title, Some("Inception".to_string()));
        assert_eq!(response.year, Some("2010".to_string()));
        assert_eq!(response.media_type, Some("movie".to_string()));
        assert_eq!(response.imdb_id, Some("tt1375666".to_string()));
    }

    #[test]
    fn test_omdb_error_response() {
        let json = r#"{
            "Response": "False",
            "Error": "Movie not found!"
        }"#;

        let response: OmdbResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.response, "False");
        assert_eq!(response.error, Some("Movie not found!".to_string()));
    }

    #[test]
    fn test_parse_response_movie() {
        let provider = OmdbProvider::new("test_key");
        let response = OmdbResponse {
            title: Some("The Matrix".to_string()),
            year: Some("1999".to_string()),
            media_type: Some("movie".to_string()),
            poster: Some("https://example.com/poster.jpg".to_string()),
            plot: Some("A computer hacker learns the truth.".to_string()),
            imdb_id: Some("tt0133093".to_string()),
            response: "True".to_string(),
            error: None,
        };

        let media_info = provider.parse_response(response).unwrap();
        assert_eq!(media_info.title, "The Matrix");
        assert_eq!(media_info.year, Some(1999));
        assert_eq!(media_info.media_type, MediaType::Movie);
        assert_eq!(media_info.imdb_id, Some("tt0133093".to_string()));
        assert_eq!(media_info.source, "OMDb");
    }

    #[test]
    fn test_parse_response_tv_series() {
        let provider = OmdbProvider::new("test_key");
        let response = OmdbResponse {
            title: Some("Breaking Bad".to_string()),
            year: Some("2008–2013".to_string()), // Note the en-dash
            media_type: Some("series".to_string()),
            poster: None,
            plot: Some("A chemistry teacher turned meth cook.".to_string()),
            imdb_id: Some("tt0903747".to_string()),
            response: "True".to_string(),
            error: None,
        };

        let media_info = provider.parse_response(response).unwrap();
        assert_eq!(media_info.title, "Breaking Bad");
        assert_eq!(media_info.year, Some(2008)); // Should parse first year
        assert_eq!(media_info.media_type, MediaType::TVShow);
    }

    #[test]
    fn test_parse_response_na_poster() {
        let provider = OmdbProvider::new("test_key");
        let response = OmdbResponse {
            title: Some("Obscure Movie".to_string()),
            year: Some("2000".to_string()),
            media_type: Some("movie".to_string()),
            poster: Some("N/A".to_string()),
            plot: Some("N/A".to_string()),
            imdb_id: Some("tt1234567".to_string()),
            response: "True".to_string(),
            error: None,
        };

        let media_info = provider.parse_response(response).unwrap();
        assert_eq!(media_info.poster_url, None);
        assert_eq!(media_info.description, None);
    }
}
