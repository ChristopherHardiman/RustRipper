use colored::Colorize;
use log::info;
use rustripper_core::Config;
use std::process::Command;

pub async fn show() -> anyhow::Result<()> {
    println!("{}", "Current Configuration".bright_green().bold());
    println!("{}", "━".repeat(60).bright_cyan());
    println!();

    match Config::load() {
        Ok(config) => {
            print_config(&config);
            Ok(())
        }
        Err(e) => {
            println!("{} Configuration file not found or invalid", "⚠".bright_yellow().bold());
            println!("  Error: {}", format!("{}", e).bright_red());
            println!();
            println!("  Run {} to create default configuration", "rustripper config init".bright_cyan());
            Ok(())
        }
    }
}

pub async fn edit() -> anyhow::Result<()> {
    let config_path = Config::config_path()?;
    
    println!("{}", "Opening configuration file...".bright_green().bold());
    println!("  Path: {}", config_path.display().to_string().bright_yellow());
    println!();

    if !config_path.exists() {
        println!("{} Configuration file does not exist", "⚠".bright_yellow().bold());
        println!("  Creating default configuration...");
        let config = Config::default();
        config.save(&config_path)?;
        println!("{} Default configuration created", "✓".bright_green().bold());
        println!();
    }

    // Try to open with default editor
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    
    info!("Opening config with editor: {}", editor);
    
    let status = Command::new(&editor)
        .arg(&config_path)
        .status()?;

    if status.success() {
        println!("{} Configuration file saved", "✓".bright_green().bold());
        println!("  Changes will take effect immediately");
    } else {
        println!("{} Editor exited with error", "✗".bright_red().bold());
    }

    Ok(())
}

pub async fn set(key: &str, value: &str) -> anyhow::Result<()> {
    println!("{}", "Updating configuration...".bright_green().bold());
    println!("  Key: {}", key.bright_cyan());
    println!("  Value: {}", value.bright_yellow());
    println!();

    let mut config = Config::load().unwrap_or_default();

    // Update the appropriate field
    match key {
        "output_directory" => config.output_directory = Some(value.to_string()),
        "disc_device" => config.disc_device = Some(value.to_string()),
        "makemkv_executable" => config.makemkv_executable = Some(value.to_string()),
        "makemkv_min_title_length" => {
            config.makemkv_min_title_length = Some(value.parse()?);
        }
        "ffmpeg_executable" => config.ffmpeg_executable = Some(value.to_string()),
        "ffmpeg_preset" => config.ffmpeg_preset = Some(value.to_string()),
        "ffmpeg_crf" => {
            config.ffmpeg_crf = Some(value.parse()?);
        }
        "metadata_tmdb_api_key" => config.metadata_tmdb_api_key = Some(value.to_string()),
        "metadata_omdb_api_key" => config.metadata_omdb_api_key = Some(value.to_string()),
        _ => {
            println!("{} Unknown configuration key: {}", "✗".bright_red().bold(), key.bright_red());
            println!();
            println!("Valid keys:");
            println!("  • output_directory");
            println!("  • disc_device");
            println!("  • makemkv_executable");
            println!("  • makemkv_min_title_length");
            println!("  • ffmpeg_executable");
            println!("  • ffmpeg_preset");
            println!("  • ffmpeg_crf");
            println!("  • metadata_tmdb_api_key");
            println!("  • metadata_omdb_api_key");
            anyhow::bail!("Unknown configuration key: {}", key);
        }
    }

    config.save()?;
    info!("Configuration updated: {} = {}", key, value);

    println!("{} Configuration updated successfully", "✓".bright_green().bold());
    println!();

    Ok(())
}

pub async fn get(key: &str) -> anyhow::Result<()> {
    let config = Config::load().unwrap_or_default();

    let value = match key {
        "output_directory" => config.output_directory.unwrap_or_else(|| "not set".to_string()),
        "disc_device" => config.disc_device.unwrap_or_else(|| "/dev/sr0".to_string()),
        "makemkv_executable" => config.makemkv_executable.unwrap_or_else(|| "makemkvcon".to_string()),
        "makemkv_min_title_length" => config.makemkv_min_title_length.unwrap_or(180).to_string(),
        "ffmpeg_executable" => config.ffmpeg_executable.unwrap_or_else(|| "ffmpeg".to_string()),
        "ffmpeg_preset" => config.ffmpeg_preset.unwrap_or_else(|| "balanced".to_string()),
        "ffmpeg_crf" => config.ffmpeg_crf.unwrap_or(20).to_string(),
        "metadata_tmdb_api_key" => {
            let key = config.metadata_tmdb_api_key.unwrap_or_else(|| "not set".to_string());
            if key.is_empty() || key == "not set" {
                "not set".to_string()
            } else {
                format!("{}...{}", &key[..4.min(key.len())], if key.len() > 8 { &key[key.len()-4..] } else { "" })
            }
        }
        "metadata_omdb_api_key" => {
            let key = config.metadata_omdb_api_key.unwrap_or_else(|| "not set".to_string());
            if key.is_empty() || key == "not set" {
                "not set".to_string()
            } else {
                format!("{}...{}", &key[..4.min(key.len())], if key.len() > 8 { &key[key.len()-4..] } else { "" })
            }
        }
        _ => {
            println!("{} Unknown configuration key: {}", "✗".bright_red().bold(), key.bright_red());
            anyhow::bail!("Unknown configuration key: {}", key);
        }
    };

    println!("{} {}", key.bright_cyan(), value.bright_white());

    Ok(())
}

pub async fn init() -> anyhow::Result<()> {
    println!("{}", "Initializing default configuration...".bright_green().bold());
    println!();

    let config_path = Config::config_path()?;

    if config_path.exists() {
        println!("{} Configuration file already exists", "⚠".bright_yellow().bold());
        println!("  Path: {}", config_path.display().to_string().bright_yellow());
        println!();
        print!("  Overwrite existing configuration? [y/N]: ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("  Cancelled");
            return Ok(());
        }
    }

    let config = Config::default();
    config.save()?;

    println!("{} Default configuration created", "✓".bright_green().bold());
    println!("  Path: {}", config_path.display().to_string().bright_yellow());
    println!();
    print_config(&config);

    info!("Default configuration initialized at: {}", config_path.display());

    Ok(())
}

fn print_config(config: &Config) {
    println!("{}", "[Paths]".bright_magenta().bold());
    println!("  output_directory = {}", 
        config.general.output_dir.display().to_string().bright_yellow());
    println!("  disc_device = {}", 
        config.general.disc_device.bright_yellow());
    println!();

    println!("{}", "[MakeMKV]".bright_magenta().bold());
    println!("  executable = {}", 
        config.makemkv.executable.bright_yellow());
    println!("  min_title_length = {} seconds", 
        config.makemkv.min_title_length.to_string().bright_yellow());
    println!();

    println!("{}", "[FFmpeg]".bright_magenta().bold());
    println!("  executable = {}", 
        config.ffmpeg.executable.bright_yellow());
    println!("  preset = {}", 
        config.ffmpeg.preset.bright_yellow());
    println!("  crf = {}", 
        config.ffmpeg.crf.to_string().bright_yellow());
    println!();

    println!("{}", "[Metadata]".bright_magenta().bold());
    let tmdb_status = if let Some(ref key) = config.metadata.tmdb_api_key {
        if key.is_empty() { "not set".bright_red() } else { "configured".bright_green() }
    } else {
        "not set".bright_red()
    };
    println!("  tmdb_api_key = {}", tmdb_status);

    let omdb_status = if let Some(ref key) = config.metadata.omdb_api_key {
        if key.is_empty() { "not set".bright_red() } else { "configured".bright_green() }
    } else {
        "not set".bright_red()
    };
    println!("  omdb_api_key = {}", omdb_status);
    println!();

    println!("{}", "━".repeat(60).bright_cyan());
}
