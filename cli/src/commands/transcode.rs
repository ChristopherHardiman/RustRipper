use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info};
use rustripper_transcode::{FFmpegTranscoder, TranscodePreset, HardwareAccel};
use std::path::Path;

pub async fn execute(
    input: &str,
    output: &str,
    preset: &str,
    crf: Option<u8>,
    thumbnail_time: Option<f64>,
) -> anyhow::Result<()> {
    let input_path = Path::new(input);
    let output_path = Path::new(output);

    if !input_path.exists() {
        error!("Input file not found: {}", input);
        println!("{} Input file not found: {}", "✗".bright_red().bold(), input.bright_red());
        anyhow::bail!("Input file does not exist: {}", input);
    }

    // Handle thumbnail generation
    if let Some(time) = thumbnail_time {
        return generate_thumbnail(input_path, output_path, time).await;
    }

    println!("{}", "Starting transcoding...".bright_green().bold());
    println!("  Input: {}", input.bright_yellow());
    println!("  Output: {}", output.bright_yellow());
    println!("  Preset: {}", preset.bright_cyan());
    if let Some(crf_val) = crf {
        println!("  CRF: {}", crf_val.to_string().bright_cyan());
    }
    println!();

    // Probe input file
    println!("{}", "🔍 Analyzing input file...".bright_blue().bold());
    let transcoder = FFmpegTranscoder::new("ffmpeg");
    
    match transcoder.probe(input_path) {
        Ok(info) => {
            println!("{} Media information:", "✓".bright_green().bold());
            println!("  Duration: {:.2} minutes", (info.duration / 60.0).to_string().parse::<f64>().unwrap_or(0.0));
            println!("  Codec: {}", info.video_codec.bright_white());
            println!("  Resolution: {}x{}", info.width, info.height);
            println!("  Bitrate: {:.2} Mbps", (info.bitrate as f64 / 1_000_000.0));
            println!("  File size: {:.2} GB", (info.file_size as f64 / 1_073_741_824.0));
            println!();
        }
        Err(e) => {
            error!("Failed to probe input file: {}", e);
            println!("{} {}", "⚠ Could not analyze file:".bright_yellow().bold(), e);
            println!("  Continuing with transcoding...");
            println!();
        }
    }

    // Parse preset
    let transcode_preset = match preset.to_lowercase().as_str() {
        "balanced" => TranscodePreset::Balanced,
        "high-quality" | "highquality" => TranscodePreset::HighQuality,
        "fast" => TranscodePreset::Fast,
        "compatible" => TranscodePreset::Compatible,
        "hardware-auto" | "hardwareauto" | "hardware" => TranscodePreset::HardwareAuto,
        "passthrough" | "copy" => TranscodePreset::PassThrough,
        _ => {
            error!("Unknown preset: {}", preset);
            println!("{} Unknown preset: {}", "✗".bright_red().bold(), preset.bright_red());
            println!("  Valid presets: balanced, high-quality, fast, compatible, hardware-auto, passthrough");
            anyhow::bail!("Unknown preset: {}", preset);
        }
    };

    // Configure transcoder
    let mut transcoder = FFmpegTranscoder::new("ffmpeg")
        .with_preset(transcode_preset);

    if let Some(crf_val) = crf {
        transcoder = transcoder.with_crf(crf_val);
    }

    // Detect hardware acceleration if using hardware preset
    if matches!(transcode_preset, TranscodePreset::HardwareAuto) {
        println!("{}", "🔍 Detecting hardware acceleration...".bright_blue().bold());
        match transcoder.detect_hardware_accel() {
            Ok(hw_list) => {
                if hw_list.is_empty() {
                    println!("{} No hardware acceleration available", "⚠".bright_yellow().bold());
                    println!("  Falling back to software encoding");
                } else {
                    println!("{} Available hardware encoders:", "✓".bright_green().bold());
                    for hw in hw_list {
                        let hw_str = match hw {
                            HardwareAccel::Nvenc => "NVIDIA NVENC".bright_green(),
                            HardwareAccel::QuickSync => "Intel QuickSync".bright_blue(),
                            HardwareAccel::Amf => "AMD AMF".bright_red(),
                        };
                        println!("  • {}", hw_str);
                    }
                }
                println!();
            }
            Err(e) => {
                error!("Hardware detection failed: {}", e);
                println!("{} {}", "⚠ Hardware detection failed:".bright_yellow().bold(), e);
                println!();
            }
        }
    }

    // Start transcoding
    println!("{}", "🎬 Starting transcoding...".bright_green().bold());
    info!("Transcoding: {} -> {}, preset={:?}, crf={:?}", input, output, transcode_preset, crf);

    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{bar:40.cyan/blue}] {pos:>3}% - {eta} remaining\nFrame: {prefix} | Speed: {wide_msg}")
            .unwrap()
            .progress_chars("█▓▒░-"),
    );
    pb.set_message("Transcoding...");

    let result = transcoder.transcode(
        input_path,
        output_path,
        Some(|progress| {
            pb.set_position(progress.percentage as u64);
            pb.set_prefix(format!("{} @ {:.1} fps", progress.frame, progress.fps));
            pb.set_message(format!("{:.2}x", progress.speed));
        }),
    );

    match result {
        Ok(_) => {
            pb.finish_and_clear();
            println!();
            println!("{}", "━".repeat(60).bright_green());
            println!("{} {}", "✓ Transcoding completed!".bright_green().bold(), "🎉".bright_green());
            println!("{}", "━".repeat(60).bright_green());
            println!("  Output: {}", output.bright_yellow());
            
            // Show output file info
            if let Ok(metadata) = std::fs::metadata(output_path) {
                let size_gb = metadata.len() as f64 / 1_073_741_824.0;
                println!("  Size: {:.2} GB", size_gb);
            }
            
            println!();
            info!("Transcoding completed successfully: {}", output);
            Ok(())
        }
        Err(e) => {
            pb.abandon();
            println!();
            error!("Transcoding failed: {}", e);
            println!("{}", "━".repeat(60).bright_red());
            println!("{} {}", "✗ Transcoding failed".bright_red().bold(), "❌".bright_red());
            println!("{}", "━".repeat(60).bright_red());
            println!("  Error: {}", e.to_string().bright_red());
            println!();
            Err(e.into())
        }
    }
}

async fn generate_thumbnail(
    input: &Path,
    output: &Path,
    time: f64,
) -> anyhow::Result<()> {
    println!("{}", "Generating thumbnail...".bright_green().bold());
    println!("  Input: {}", input.display().to_string().bright_yellow());
    println!("  Output: {}", output.display().to_string().bright_yellow());
    println!("  Time: {:.2} seconds", time);
    println!();

    let transcoder = FFmpegTranscoder::new("ffmpeg");
    
    info!("Generating thumbnail at {:.2}s: {} -> {}", time, input.display(), output.display());
    
    match transcoder.generate_thumbnail(input, output, time) {
        Ok(_) => {
            println!("{}", "━".repeat(60).bright_green());
            println!("{} {}", "✓ Thumbnail generated!".bright_green().bold(), "🖼️".bright_green());
            println!("{}", "━".repeat(60).bright_green());
            println!("  Output: {}", output.display().to_string().bright_yellow());
            println!();
            info!("Thumbnail generated successfully");
            Ok(())
        }
        Err(e) => {
            error!("Thumbnail generation failed: {}", e);
            println!("{}", "━".repeat(60).bright_red());
            println!("{} {}", "✗ Thumbnail generation failed".bright_red().bold(), "❌".bright_red());
            println!("{}", "━".repeat(60).bright_red());
            println!("  Error: {}", e.to_string().bright_red());
            println!();
            Err(e.into())
        }
    }
}
