use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info};
use rustripper_disc::DiscWatcher;
use rustripper_metadata::MetadataAggregator;
use rustripper_ripper::MakeMKVRipper;
use std::path::Path;

pub async fn execute(
    device: &str,
    output_dir: &str,
    title: &str,
    min_length: u32,
    fetch_metadata: bool,
) -> anyhow::Result<()> {
    println!("{}", "Starting manual rip...".bright_green().bold());
    println!("  Device: {}", device.bright_yellow());
    println!("  Output: {}", output_dir.bright_yellow());
    println!("  Title selection: {}", title.bright_cyan());
    println!("  Min length: {} seconds", min_length.to_string().bright_cyan());
    println!("  Metadata lookup: {}", if fetch_metadata { "enabled".bright_green() } else { "disabled".bright_red() });
    println!();

    // Check if disc is present
    println!("{}", "🔍 Checking for disc...".bright_blue().bold());
    let mut watcher = DiscWatcher::new(device);
    
    let disc = match watcher.poll()? {
        Some(disc) => {
            println!("{} {}", "✓ Disc detected:".bright_green().bold(), disc.label.bright_white().bold());
            println!("  Type: {}", format!("{:?}", disc.disc_type).bright_yellow());
            disc
        }
        None => {
            error!("No disc detected in {}", device);
            println!("{}", "✗ No disc found in drive".bright_red().bold());
            anyhow::bail!("No disc present in {}", device);
        }
    };
    println!();

    // Fetch metadata if requested
    let mut title_name = disc.label.clone();
    if fetch_metadata {
        println!("{}", "🔍 Fetching metadata...".bright_blue().bold());
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
        );
        spinner.set_message("Searching metadata providers...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));

        match fetch_disc_metadata(&disc.label).await {
            Ok(Some(media)) => {
                spinner.finish_and_clear();
                println!("{} Found metadata!", "✓".bright_green().bold());
                println!("  Title: {}", media.title.bright_white().bold());
                if let Some(year) = media.year {
                    println!("  Year: {}", year.to_string().bright_yellow());
                    title_name = format!("{} ({})", media.title, year);
                } else {
                    title_name = media.title.clone();
                }
                if let Some(media_type) = media.media_type {
                    println!("  Type: {}", format!("{:?}", media_type).bright_cyan());
                }
                println!();
            }
            Ok(None) => {
                spinner.finish_and_clear();
                println!("{} No metadata found, using disc label", "⚠".bright_yellow().bold());
                println!();
            }
            Err(e) => {
                spinner.finish_and_clear();
                error!("Metadata fetch error: {}", e);
                println!("{} {}", "⚠ Metadata error:".bright_yellow().bold(), e);
                println!("  Continuing with disc label...");
                println!();
            }
        }
    }

    // Create output directory if it doesn't exist
    let output_path = Path::new(output_dir);
    if !output_path.exists() {
        std::fs::create_dir_all(output_path)?;
        info!("Created output directory: {}", output_dir);
        println!("{} {}", "✓ Created directory:".bright_green(), output_dir.bright_yellow());
        println!();
    }

    // Start ripping
    println!("{}", "🎬 Starting MakeMKV rip...".bright_green().bold());
    info!("Starting rip: device={}, output={}, title={}, min_length={}", device, output_dir, title, min_length);

    let ripper = MakeMKVRipper::new("makemkvcon", min_length)
        .with_title_selection(title);

    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{bar:40.cyan/blue}] {pos:>3}% - {eta} remaining")
            .unwrap()
            .progress_chars("█▓▒░-"),
    );
    pb.set_message(format!("Ripping: {}", title_name));

    let result = ripper.rip(
        device,
        output_path,
        Some(|progress, message| {
            pb.set_position(progress as u64);
            if !message.is_empty() {
                pb.set_message(format!("Ripping: {} - {}", title_name, message));
            }
        }),
    );

    match result {
        Ok(_) => {
            pb.finish_and_clear();
            println!();
            println!("{}", "━".repeat(60).bright_green());
            println!("{} {}", "✓ Rip completed successfully!".bright_green().bold(), "🎉".bright_green());
            println!("{}", "━".repeat(60).bright_green());
            println!("  Title: {}", title_name.bright_white().bold());
            println!("  Output: {}", output_dir.bright_yellow());
            println!();
            info!("Rip completed successfully: {}", title_name);
            Ok(())
        }
        Err(e) => {
            pb.abandon();
            println!();
            error!("Rip failed: {}", e);
            println!("{}", "━".repeat(60).bright_red());
            println!("{} {}", "✗ Rip failed".bright_red().bold(), "❌".bright_red());
            println!("{}", "━".repeat(60).bright_red());
            println!("  Error: {}", e.to_string().bright_red());
            println!();
            Err(e.into())
        }
    }
}

async fn fetch_disc_metadata(disc_label: &str) -> anyhow::Result<Option<rustripper_core::MediaInfo>> {
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

    let results = aggregator.search_with_type_detection(disc_label).await?;
    Ok(results.into_iter().next())
}
