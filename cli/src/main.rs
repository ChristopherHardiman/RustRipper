use clap::{Parser, Subcommand};
use colored::Colorize;

mod commands;

#[derive(Parser)]
#[command(name = "rustripper")]
#[command(about = "MasterRustRipper - Automated DVD/Blu-ray ripping and transcoding", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Monitor optical drive and auto-rip on disc insertion
    Watch {
        /// Optical drive device path
        #[arg(short, long, default_value = "/dev/sr0")]
        device: String,

        /// Output directory for ripped files
        #[arg(short, long)]
        output: Option<String>,

        /// Auto-start ripping when disc detected
        #[arg(short, long)]
        auto_rip: bool,
    },

    /// Manually rip current disc
    Rip {
        /// Optical drive device path
        #[arg(short, long, default_value = "/dev/sr0")]
        device: String,

        /// Output directory
        #[arg(short, long, required = true)]
        output: String,

        /// Title selection ("all" or specific title number)
        #[arg(short, long, default_value = "all")]
        title: String,

        /// Minimum title length in seconds
        #[arg(short, long, default_value = "180")]
        min_length: u32,

        /// Skip metadata lookup
        #[arg(long)]
        no_metadata: bool,
    },

    /// Search metadata providers
    Search {
        /// Search query
        query: String,

        /// Filter by year
        #[arg(short, long)]
        year: Option<u32>,

        /// Specific provider (tmdb, anilist, omdb)
        #[arg(short, long)]
        provider: Option<String>,
    },

    /// Transcode video file
    Transcode {
        /// Input file path
        input: String,

        /// Output file path
        output: String,

        /// Transcode preset (balanced, high-quality, fast, compatible, hardware-auto, passthrough)
        #[arg(short, long, default_value = "balanced")]
        preset: String,

        /// CRF value (0-51, lower = better quality)
        #[arg(short, long)]
        crf: Option<u8>,

        /// Generate thumbnail at specified time (seconds)
        #[arg(long)]
        thumbnail: Option<f64>,
    },

    /// View or edit configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,

    /// Edit configuration file
    Edit,

    /// Set configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,
    },

    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },

    /// Initialize default configuration
    Init,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Print banner
    println!("{}", "=".repeat(60).bright_cyan());
    println!(
        "{}",
        "  MasterRustRipper - DVD/Blu-ray Automation Suite  ".bright_cyan().bold()
    );
    println!("{}", "=".repeat(60).bright_cyan());
    println!();

    // Execute command
    match cli.command {
        Commands::Watch {
            device,
            output,
            auto_rip,
        } => commands::watch::execute(&device, output.as_deref(), auto_rip).await,

        Commands::Rip {
            device,
            output,
            title,
            min_length,
            no_metadata,
        } => commands::rip::execute(&device, &output, &title, min_length, !no_metadata).await,

        Commands::Search {
            query,
            year,
            provider,
        } => commands::search::execute(&query, year, provider.as_deref()).await,

        Commands::Transcode {
            input,
            output,
            preset,
            crf,
            thumbnail,
        } => commands::transcode::execute(&input, &output, &preset, crf, thumbnail).await,

        Commands::Config { action } => match action {
            ConfigAction::Show => commands::config::show().await,
            ConfigAction::Edit => commands::config::edit().await,
            ConfigAction::Set { key, value } => commands::config::set(&key, &value).await,
            ConfigAction::Get { key } => commands::config::get(&key).await,
            ConfigAction::Init => commands::config::init().await,
        },
    }
}
