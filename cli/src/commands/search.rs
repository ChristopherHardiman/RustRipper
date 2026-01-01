use colored::Colorize;
use log::{error, info};
use rustripper_metadata::{MetadataAggregator, TmdbProvider, AnilistProvider, OmdbProvider};

pub async fn execute(
    query: &str,
    year: Option<u32>,
    provider: Option<&str>,
) -> anyhow::Result<()> {
    println!("{}", "Searching metadata providers...".bright_green().bold());
    println!("  Query: {}", query.bright_white().bold());
    if let Some(y) = year {
        println!("  Year: {}", y.to_string().bright_yellow());
    }
    if let Some(p) = provider {
        println!("  Provider: {}", p.bright_cyan());
    }
    println!();

    info!("Search query: {}, year: {:?}, provider: {:?}", query, year, provider);

    let results = match provider {
        Some("tmdb") => search_tmdb(query, year).await?,
        Some("anilist") => search_anilist(query, year).await?,
        Some("omdb") => search_omdb(query, year).await?,
        Some(p) => {
            error!("Unknown provider: {}", p);
            println!("{} Unknown provider: {}", "✗".bright_red().bold(), p.bright_red());
            anyhow::bail!("Unknown provider: {}. Valid options: tmdb, anilist, omdb", p);
        }
        None => search_all(query, year).await?,
    };

    if results.is_empty() {
        println!("{} No results found", "⚠".bright_yellow().bold());
        return Ok(());
    }

    println!("{}", "━".repeat(80).bright_cyan());
    println!("{} {} result{}", "✓".bright_green().bold(), results.len(), if results.len() == 1 { "" } else { "s" });
    println!("{}", "━".repeat(80).bright_cyan());
    println!();

    for (idx, media) in results.iter().enumerate() {
        println!("{} {}", format!("[{}]", idx + 1).bright_cyan().bold(), media.title.bright_white().bold());
        
        if let Some(year) = media.year {
            println!("  {} {}", "Year:".bright_blue(), year.to_string().bright_yellow());
        }
        
        if let Some(media_type) = &media.media_type {
            println!("  {} {}", "Type:".bright_blue(), format!("{:?}", media_type).bright_green());
        }
        
        if let Some(poster) = &media.poster_url {
            println!("  {} {}", "Poster:".bright_blue(), poster.bright_cyan());
        }
        
        if let Some(imdb_id) = &media.imdb_id {
            println!("  {} {}", "IMDb:".bright_blue(), format!("https://www.imdb.com/title/{}", imdb_id).bright_cyan());
        }
        
        if let Some(desc) = &media.description {
            let truncated = if desc.len() > 150 {
                format!("{}...", &desc[..147])
            } else {
                desc.clone()
            };
            println!("  {} {}", "Description:".bright_blue(), truncated.bright_white());
        }
        
        println!();
    }

    println!("{}", "━".repeat(80).bright_cyan());
    info!("Search completed: {} result(s)", results.len());
    
    Ok(())
}

async fn search_all(query: &str, year: Option<u32>) -> anyhow::Result<Vec<rustripper_core::MediaInfo>> {
    let config = rustripper_core::Config::load().ok();
    let mut aggregator = MetadataAggregator::new();
    
    if let Some(cfg) = &config {
        if let Some(tmdb_key) = &cfg.metadata_tmdb_api_key {
            if !tmdb_key.is_empty() {
                aggregator = aggregator.with_tmdb(tmdb_key);
            }
        }
        if let Some(omdb_key) = &cfg.metadata_omdb_api_key {
            if !omdb_key.is_empty() {
                aggregator = aggregator.with_omdb(omdb_key);
            }
        }
    }

    let results = if year.is_some() {
        aggregator.search(query, year).await?
    } else {
        aggregator.search_with_type_detection(query).await?
    };

    Ok(results)
}

async fn search_tmdb(query: &str, year: Option<u32>) -> anyhow::Result<Vec<rustripper_core::MediaInfo>> {
    let config = rustripper_core::Config::load()?;
    let api_key = config.metadata_tmdb_api_key
        .ok_or_else(|| anyhow::anyhow!("TMDb API key not configured. Run: rustripper config set metadata_tmdb_api_key YOUR_KEY"))?;

    if api_key.is_empty() {
        anyhow::bail!("TMDb API key is empty. Run: rustripper config set metadata_tmdb_api_key YOUR_KEY");
    }

    let provider = TmdbProvider::new(&api_key);
    let results = provider.search(query, year).await?;
    Ok(results)
}

async fn search_anilist(query: &str, year: Option<u32>) -> anyhow::Result<Vec<rustripper_core::MediaInfo>> {
    let provider = AnilistProvider::new();
    let results = provider.search(query, year).await?;
    Ok(results)
}

async fn search_omdb(query: &str, year: Option<u32>) -> anyhow::Result<Vec<rustripper_core::MediaInfo>> {
    let config = rustripper_core::Config::load()?;
    let api_key = config.metadata_omdb_api_key
        .ok_or_else(|| anyhow::anyhow!("OMDb API key not configured. Run: rustripper config set metadata_omdb_api_key YOUR_KEY"))?;

    if api_key.is_empty() {
        anyhow::bail!("OMDb API key is empty. Run: rustripper config set metadata_omdb_api_key YOUR_KEY");
    }

    let provider = OmdbProvider::new(&api_key);
    let results = provider.search(query, year).await?;
    Ok(results)
}
