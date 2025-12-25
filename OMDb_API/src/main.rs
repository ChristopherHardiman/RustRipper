use reqwest::Error;
mod omdb_api;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let api_key = omdb_api::read_api_key_from_config().expect("Failed to read API key from conf.toml");
    let movie_title = "Inception";

    let movie_data = omdb_api::fetch_movie_data(&api_key, movie_title).await?;
    println!("Movie data: {:?}", movie_data);

    Ok(())
}


