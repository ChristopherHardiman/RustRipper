use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info, warn};
use rustripper_disc::DiscWatcher;
use rustripper_metadata::MetadataAggregator;
use rustripper_ripper::MakeMKVRipper;
use std::path::Path;
use std::time::Duration;

pub async fn execute(device: &str, output_dir: Option<&str>, auto_rip: bool) -> anyhow::Result<()> {
    println!("{}", "Starting disc watcher...".bright_green().bold());
    println!("  Device: {}", device.bright_yellow());
    println!("  Auto-rip: {}", if auto_rip { "enabled".bright_green() } else { "disabled".bright_red() });
    
    if let Some(output) = output_dir {
        println!("  Output: {}", output.bright_yellow());
    }
    println!();

    let mut watcher = DiscWatcher::new(device);
    let mut last_label: Option<String> = None;

    info!("Monitoring {} for disc insertion...", device);
    println!("{}", "Waiting for disc insertion... (Press Ctrl+C to exit)".bright_cyan());
    println!();

    loop {
        match watcher.poll() {
            Ok(Some(disc)) => {
                // Check if this is a new disc (different label)
                let is_new = last_label.as_ref() != Some(&disc.label);
                
                if is_new {
                    println!("{}", "━".repeat(60).bright_cyan());
                    println!("{} {}", "📀 Disc detected:".bright_green().bold(), disc.label.bright_white().bold());
                    println!("  Type: {}", format!("{:?}", disc.disc_type).bright_yellow());
                    println!("{}", "━".repeat(60).bright_cyan());
                    
                    info!("Disc inserted: {} ({:?})", disc.label, disc.disc_type);
                    
                    // Fetch metadata
                    println!("\n{}", "🔍 Fetching metadata...".bright_blue().bold());
                    let spinner = ProgressBar::new_spinner();
                    spinner.set_style(
                        ProgressStyle::default_spinner()
                            .template("{spinner:.cyan} {msg}")
                            .unwrap()
                    );
                    spinner.set_message("Searching metadata providers...");
                    spinner.enable_steady_tick(Duration::from_millis(100));

                    match fetch_metadata(&disc.label).await {
                        Ok(Some(media)) => {
                            spinner.finish_and_clear();
                            println!("{} Found metadata!", "✓".bright_green().bold());
                            println!("  Title: {}", media.title.bright_white().bold());
                            if let Some(year) = media.year {
                                println!("  Year: {}", year.to_string().bright_yellow());
                            }
                            println!("  Type: {}", format!("{:?}", media.media_type).bright_cyan());
                            println!();

                            if auto_rip {
                                if let Some(output) = output_dir {
                                    println!("{}", "🎬 Auto-ripping enabled, starting rip...".bright_green().bold());
                                    if let Err(e) = perform_rip(device, output, &disc.label).await {
                                        error!("Rip failed: {}", e);
                                        println!("{} {}", "✗ Rip failed:".bright_red().bold(), e);
                                    }
                                } else {
                                    warn!("Auto-rip enabled but no output directory specified");
                                    println!("{}", "⚠ Auto-rip enabled but no output directory specified".bright_yellow());
                                }
                            } else {
                                println!("{}", "ℹ Auto-rip disabled. Use 'rustripper rip' to start manually.".bright_cyan());
                            }
                        }
                        Ok(None) => {
                            spinner.finish_and_clear();
                            warn!("No metadata found for: {}", disc.label);
                            println!("{} No metadata found", "⚠".bright_yellow().bold());
                            
                            if auto_rip {
                                println!("{}", "  Continuing with disc label as title...".bright_yellow());
                                if let Some(output) = output_dir {
                                    if let Err(e) = perform_rip(device, output, &disc.label).await {
                                        error!("Rip failed: {}", e);
                                        println!("{} {}", "✗ Rip failed:".bright_red().bold(), e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            spinner.finish_and_clear();
                            error!("Metadata fetch error: {}", e);
                            println!("{} {}", "✗ Metadata error:".bright_red().bold(), e);
                        }
                    }
                    
                    last_label = Some(disc.label.clone());
                    println!();
                }
            }
            Ok(None) => {
                // No disc present
                if last_label.is_some() {
                    println!("{}", "📤 Disc ejected".bright_yellow());
                    info!("Disc ejected");
                    last_label = None;
                    println!();
                }
            }
            Err(e) => {
                error!("Disc detection error: {}", e);
                // Don't print error every poll, just log it
            }
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn fetch_metadata(disc_label: &str) -> anyhow::Result<Option<rustripper_core::MediaInfo>> {
    // Try to load config for API keys
    let config = rustripper_core::Config::load().ok();
    
    let mut aggregator = MetadataAggregator::new();
    
    if let Some(cfg) = &config {
            if let Some(ref tmdb_key) = cfg.metadata.tmdb_api_key {
                if !tmdb_key.is_empty() {
                    aggregator = aggregator.with_tmdb(tmdb_key);
                }
            }
            if let Some(ref omdb_key) = cfg.metadata.omdb_api_key {
                if !omdb_key.is_empty() {
                    aggregator = aggregator.with_omdb(omdb_key);
                }
            }
    }

    let results = aggregator.search_with_type_detection(disc_label).await?;
    Ok(results.into_iter().next())
}

async fn perform_rip(device: &str, output_dir: &str, disc_label: &str) -> anyhow::Result<()> {
    println!("{}", "🎬 Starting rip process...".bright_green().bold());
    
    let output_path = Path::new(output_dir);
    
    // Create output directory if it doesn't exist
    if !output_path.exists() {
        std::fs::create_dir_all(output_path)?;
        println!("  Created output directory: {}", output_dir.bright_yellow());
    }

    let ripper = MakeMKVRipper::new("makemkvcon", 180)
        .with_title_selection("all");

    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{bar:40.cyan/blue}] {pos:>3}% ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(format!("Ripping: {}", disc_label));

    let result = ripper.rip(
        device,
        output_path,
        Some(|progress, message| {
            pb.set_position(progress as u64);
            if !message.is_empty() {
                pb.set_message(format!("Ripping: {} - {}", disc_label, message));
            }
        }),
    );

    match result {
        Ok(_) => {
            pb.finish_with_message(format!("✓ Rip completed: {}", disc_label));
            info!("Rip completed successfully: {}", disc_label);
            println!();
            Ok(())
        }
        Err(e) => {
            pb.abandon_with_message(format!("✗ Rip failed: {}", e));
            Err(e.into())
        }
    }
}
