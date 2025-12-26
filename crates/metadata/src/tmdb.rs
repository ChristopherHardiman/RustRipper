//! TMDb (The Movie Database) API client for movies and TV shows

use rustripper_core::{MediaInfo, MediaType, Result, RipperError};
use serde::{Deserialize, Serialize};

/// TMDb API client
pub struct TmdbProvider {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
}

impl TmdbProvider {
    /// Create a new TMDb provider with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: reqwest::Client::new(),
            base_url: "https://api.themoviedb.org/3".to_string(),
        }
    }

    /// Search for movies and TV shows
    pub async fn search(&self, query: &str, year: Option<u16>) -> Result<Vec<MediaInfo>> {
        let mut results = Vec::new();

        // Search movies
        if let Ok(movies) = self.search_movies(query, year).await {
            results.extend(movies);
        }

        // Search TV shows
        if let Ok(tv_shows) = self.search_tv(query, year).await {
            results.extend(tv_shows);
        }

        if results.is_empty() {
            return Err(RipperError::metadata_error(
                query,
                "No results found on TMDb",
            ));
        }

        Ok(results)
    }

    /// Search for movies specifically
    pub async fn search_movies(&self, query: &str, year: Option<u16>) -> Result<Vec<MediaInfo>> {
        let encoded_query = urlencoding::encode(query);
        let mut url = format!(
            "{}/search/movie?api_key={}&query={}",
            self.base_url, self.api_key, encoded_query
        );

        if let Some(y) = year {
            url.push_str(&format!("&year={}", y));
        }

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RipperError::network_error(format!("TMDb request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RipperError::metadata_error(
                query,
                format!("TMDb API error: {}", response.status()),
            ));
        }

        let tmdb_response: TmdbSearchResponse = response.json().await.map_err(|e| {
            RipperError::metadata_error(query, format!("Failed to parse TMDb response: {}", e))
        })?;

        Ok(tmdb_response
            .results
            .into_iter()
            .map(|movie| movie.to_media_info())
            .collect())
    }

    /// Search for TV shows specifically
    pub async fn search_tv(&self, query: &str, year: Option<u16>) -> Result<Vec<MediaInfo>> {
        let encoded_query = urlencoding::encode(query);
        let mut url = format!(
            "{}/search/tv?api_key={}&query={}",
            self.base_url, self.api_key, encoded_query
        );

        if let Some(y) = year {
            url.push_str(&format!("&first_air_date_year={}", y));
        }

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RipperError::network_error(format!("TMDb request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RipperError::metadata_error(
                query,
                format!("TMDb API error: {}", response.status()),
            ));
        }

        let tmdb_response: TmdbTvSearchResponse = response.json().await.map_err(|e| {
            RipperError::metadata_error(query, format!("Failed to parse TMDb response: {}", e))
        })?;

        Ok(tmdb_response
            .results
            .into_iter()
            .map(|tv| tv.to_media_info())
            .collect())
    }

    /// Get movie details by ID
    pub async fn get_movie(&self, movie_id: u32) -> Result<MediaInfo> {
        let url = format!(
            "{}/movie/{}?api_key={}",
            self.base_url, movie_id, self.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RipperError::network_error(format!("TMDb request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(RipperError::MetadataError {
                query: movie_id.to_string(),
                reason: format!("TMDb API error: {}", response.status()),
            });
        }

        let movie: TmdbMovie = response.json().await.map_err(|e| {
            RipperError::MetadataError {
                query: movie_id.to_string(),
                reason: format!("Failed to parse TMDb response: {}", e),
            }
        })?;

        Ok(movie.to_media_info())
    }
}

// TMDb API response structures

#[derive(Debug, Deserialize)]
struct TmdbSearchResponse {
    results: Vec<TmdbMovie>,
}

#[derive(Debug, Deserialize)]
struct TmdbTvSearchResponse {
    results: Vec<TmdbTvShow>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TmdbMovie {
    id: u32,
    title: String,
    #[serde(rename = "release_date")]
    release_date: Option<String>,
    overview: Option<String>,
    #[serde(rename = "poster_path")]
    poster_path: Option<String>,
    #[serde(rename = "backdrop_path")]
    backdrop_path: Option<String>,
    #[serde(rename = "vote_average")]
    vote_average: Option<f32>,
    #[serde(rename = "original_language")]
    original_language: Option<String>,
}

impl TmdbMovie {
    fn to_media_info(self) -> MediaInfo {
        let year = self
            .release_date
            .as_ref()
            .and_then(|date| date.split('-').next())
            .and_then(|year_str| year_str.parse::<u16>().ok());

        let poster_url = self.poster_path.map(|path| {
            format!("https://image.tmdb.org/t/p/w500{}", path)
        });

        MediaInfo {
            id: format!("tmdb:{}", self.id),
            title: self.title,
            year,
            description: self.overview,
            media_type: MediaType::Movie,
            poster_url,
            imdb_id: None, // TMDb doesn't always provide IMDb ID in search results
            tmdb_id: Some(self.id.to_string()),
            anilist_id: None,
            source: "TMDb".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct TmdbTvShow {
    id: u32,
    name: String,
    #[serde(rename = "first_air_date")]
    first_air_date: Option<String>,
    overview: Option<String>,
    #[serde(rename = "poster_path")]
    poster_path: Option<String>,
    #[serde(rename = "backdrop_path")]
    backdrop_path: Option<String>,
    #[serde(rename = "vote_average")]
    vote_average: Option<f32>,
    #[serde(rename = "original_language")]
    original_language: Option<String>,
}

impl TmdbTvShow {
    fn to_media_info(self) -> MediaInfo {
        let year = self
            .first_air_date
            .as_ref()
            .and_then(|date| date.split('-').next())
            .and_then(|year_str| year_str.parse::<u16>().ok());

        let poster_url = self.poster_path.map(|path| {
            format!("https://image.tmdb.org/t/p/w500{}", path)
        });

        MediaInfo {
            id: format!("tmdb:{}", self.id),
            title: self.name,
            year,
            description: self.overview,
            media_type: MediaType::TVShow,
            poster_url,
            imdb_id: None,
            tmdb_id: Some(self.id.to_string()),
            anilist_id: None,
            source: "TMDb".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tmdb_provider_creation() {
        let provider = TmdbProvider::new("test_api_key");
        assert_eq!(provider.api_key, "test_api_key");
        assert_eq!(provider.base_url, "https://api.themoviedb.org/3");
    }

    #[test]
    fn test_movie_to_media_info() {
        let movie = TmdbMovie {
            id: 550,
            title: "Fight Club".to_string(),
            release_date: Some("1999-10-15".to_string()),
            overview: Some("A ticking-time-bomb insomniac...".to_string()),
            poster_path: Some("/pB8BM7pdSp6B6Ih7QZ4DrQ3PmJK.jpg".to_string()),
            backdrop_path: None,
            vote_average: Some(8.4),
            original_language: Some("en".to_string()),
        };

        let media_info = movie.to_media_info();
        assert_eq!(media_info.title, "Fight Club");
        assert_eq!(media_info.year, Some(1999));
        assert_eq!(media_info.media_type, MediaType::Movie);
        assert!(media_info.poster_url.is_some());
        assert!(media_info.poster_url.unwrap().contains("w500"));
    }

    #[test]
    fn test_tv_show_to_media_info() {
        let tv = TmdbTvShow {
            id: 1396,
            name: "Breaking Bad".to_string(),
            first_air_date: Some("2008-01-20".to_string()),
            overview: Some("A high school chemistry teacher...".to_string()),
            poster_path: Some("/ggFHVNu6YYI5L9pCfOacjizRGt.jpg".to_string()),
            backdrop_path: None,
            vote_average: Some(9.3),
            original_language: Some("en".to_string()),
        };

        let media_info = tv.to_media_info();
        assert_eq!(media_info.title, "Breaking Bad");
        assert_eq!(media_info.year, Some(2008));
        assert_eq!(media_info.media_type, MediaType::TVShow);
        assert!(media_info.poster_url.is_some());
    }

    #[test]
    fn test_movie_missing_year() {
        let movie = TmdbMovie {
            id: 123,
            title: "Unknown Movie".to_string(),
            release_date: None,
            overview: None,
            poster_path: None,
            backdrop_path: None,
            vote_average: None,
            original_language: None,
        };

        let media_info = movie.to_media_info();
        assert_eq!(media_info.title, "Unknown Movie");
        assert_eq!(media_info.year, None);
        assert_eq!(media_info.poster_url, None);
    }

    #[test]
    fn test_year_parsing() {
        let movie = TmdbMovie {
            id: 123,
            title: "Test".to_string(),
            release_date: Some("2023-05-15".to_string()),
            overview: None,
            poster_path: None,
            backdrop_path: None,
            vote_average: None,
            original_language: None,
        };

        let media_info = movie.to_media_info();
        assert_eq!(media_info.year, Some(2023));
    }

    #[test]
    fn test_poster_url_formatting() {
        let movie = TmdbMovie {
            id: 123,
            title: "Test".to_string(),
            release_date: None,
            overview: None,
            poster_path: Some("/abc123.jpg".to_string()),
            backdrop_path: None,
            vote_average: None,
            original_language: None,
        };

        let media_info = movie.to_media_info();
        assert_eq!(
            media_info.poster_url,
            Some("https://image.tmdb.org/t/p/w500/abc123.jpg".to_string())
        );
    }
}
