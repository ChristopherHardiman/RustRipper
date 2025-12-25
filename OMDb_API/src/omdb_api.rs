use reqwest::Error;
use serde::Deserialize;
use std::fs;
use toml::Value;
use std::io::{stdin, Write};


#[derive(Deserialize, Debug)]
pub struct ApiResponse {
    Title: String,
    Year: String,
    Type: String,
}

#[derive(Deserialize, Debug)]
pub struct Rating {}


pub async fn fetch_movie_data(api_key: &str, movie_title: &str) -> Result<ApiResponse, Error> {
    let url = format!(
        "http://www.omdbapi.com/?apikey={}&t={}",
        api_key,
        movie_title
    );

    let response = reqwest::get(&url).await?.json::<ApiResponse>().await?;
    Ok(response)
}

pub(crate) fn read_api_key_from_config() -> Result<String, Box<dyn std::error::Error>> {
    if !fs::metadata("conf.toml").is_ok() {
        let default_config = "api_key = \"\"\n";
        fs::write("conf.toml", default_config)?;
        println!("A default conf.toml file has been created. Please enter your OMDb API key:");
    }

    let mut config = fs::read_to_string("conf.toml")?;
    let mut config_value: Value = toml::from_str(&config)?;

    let mut api_key = config_value
        .get("api_key")
        .ok_or("API key not found in conf.toml")?
        .as_str()
        .ok_or("API key must be a string")?
        .to_owned();

    if api_key.is_empty() {
        print!("Please enter your OMDb API key: ");
        std::io::stdout().flush()?; // Ensure the prompt is displayed before reading input
        stdin().read_line(&mut api_key)?;
        api_key = api_key.trim().to_owned();
        config_value.as_table_mut().unwrap().insert("api_key".to_owned(), toml::Value::String(api_key.clone()));
        config = toml::to_string(&config_value)?;
        fs::write("conf.toml", config)?;
    }

    Ok(api_key)
}



