//! FFmpeg transcoder with preset system and hardware acceleration

use rustripper_core::{Result, RipperError};
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use serde::{Deserialize, Serialize};

/// Progress callback function type
pub type ProgressCallback = Box<dyn Fn(TranscodeProgress) + Send>;

/// Transcoding progress information
#[derive(Debug, Clone)]
pub struct TranscodeProgress {
    /// Current frame number
    pub frame: u64,
    /// Frames per second
    pub fps: f32,
    /// Current time position
    pub time: String,
    /// Current bitrate
    pub bitrate: String,
    /// Encoding speed (e.g., 2.5x)
    pub speed: f32,
    /// Progress percentage (0.0-100.0)
    pub percentage: f32,
}

/// Media information from ffprobe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    /// Duration in seconds
    pub duration: f64,
    /// Video codec name
    pub video_codec: String,
    /// Video width
    pub width: u32,
    /// Video height
    pub height: u32,
    /// Bitrate in bits/sec
    pub bitrate: u64,
    /// File size in bytes
    pub file_size: u64,
}

/// Transcoding preset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscodePreset {
    /// H.265, CRF 20, medium preset - good balance (50-60% size reduction)
    Balanced,
    /// H.265, CRF 18, slow preset - high quality (40-50% size reduction)
    HighQuality,
    /// H.265, CRF 23, fast preset - faster encoding (60-70% size reduction)
    Fast,
    /// H.264, CRF 18 - maximum device compatibility
    Compatible,
    /// Use detected GPU for hardware encoding
    HardwareAuto,
    /// Copy streams without re-encoding
    PassThrough,
}

/// Hardware acceleration type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareAccel {
    /// No hardware acceleration
    None,
    /// NVIDIA NVENC
    Nvidia,
    /// Intel QuickSync
    Intel,
    /// AMD AMF/VCE
    Amd,
    /// Auto-detect available hardware
    Auto,
}

impl std::fmt::Display for HardwareAccel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardwareAccel::None => write!(f, "None"),
            HardwareAccel::Nvidia => write!(f, "NVENC"),
            HardwareAccel::Intel => write!(f, "QuickSync"),
            HardwareAccel::Amd => write!(f, "AMF"),
            HardwareAccel::Auto => write!(f, "Auto"),
        }
    }
}

/// FFmpeg transcoder
pub struct FFmpegTranscoder {
    executable: String,
    ffprobe_executable: String,
    preset: TranscodePreset,
    crf: u8,
    hardware_accel: HardwareAccel,
}

impl FFmpegTranscoder {
    /// Create a new FFmpeg transcoder
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
            ffprobe_executable: "ffprobe".to_string(),
            preset: TranscodePreset::Balanced,
            crf: 20,
            hardware_accel: HardwareAccel::None,
        }
    }

    /// Set the transcoding preset
    pub fn with_preset(mut self, preset: TranscodePreset) -> Self {
        self.preset = preset;
        self
    }

    /// Set the quality (CRF value, 0-51)
    pub fn with_crf(mut self, crf: u8) -> Self {
        self.crf = crf.min(51);
        self
    }

    /// Set hardware acceleration
    pub fn with_hardware_accel(mut self, accel: HardwareAccel) -> Self {
        self.hardware_accel = accel;
        self
    }

    /// Probe media file for information
    pub fn probe(&self, input: &Path) -> Result<MediaInfo> {
        let output = Command::new(&self.ffprobe_executable)
            .args(&[
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                input.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    RipperError::FFmpegNotFound
                } else {
                    RipperError::FFmpegError(format!("Failed to run ffprobe: {}", e))
                }
            })?;

        if !output.status.success() {
            return Err(RipperError::FFmpegError(format!(
                "ffprobe failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        self.parse_ffprobe_output(&json_str, input)
    }

    /// Parse ffprobe JSON output
    fn parse_ffprobe_output(&self, json: &str, input: &Path) -> Result<MediaInfo> {
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| RipperError::FFmpegError(format!("Failed to parse ffprobe output: {}", e)))?;

        let format = value.get("format")
            .ok_or_else(|| RipperError::FFmpegError("Missing format in ffprobe output".to_string()))?;

        let streams = value.get("streams")
            .and_then(|s| s.as_array())
            .ok_or_else(|| RipperError::FFmpegError("Missing streams in ffprobe output".to_string()))?;

        // Find video stream
        let video_stream = streams.iter()
            .find(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("video"))
            .ok_or_else(|| RipperError::FFmpegError("No video stream found".to_string()))?;

        let duration = format.get("duration")
            .and_then(|d| d.as_str())
            .and_then(|d| d.parse::<f64>().ok())
            .unwrap_or(0.0);

        let video_codec = video_stream.get("codec_name")
            .and_then(|c| c.as_str())
            .unwrap_or("unknown")
            .to_string();

        let width = video_stream.get("width")
            .and_then(|w| w.as_u64())
            .unwrap_or(0) as u32;

        let height = video_stream.get("height")
            .and_then(|h| h.as_u64())
            .unwrap_or(0) as u32;

        let bitrate = format.get("bit_rate")
            .and_then(|b| b.as_str())
            .and_then(|b| b.parse::<u64>().ok())
            .unwrap_or(0);

        let file_size = std::fs::metadata(input)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(MediaInfo {
            duration,
            video_codec,
            width,
            height,
            bitrate,
            file_size,
        })
    }

    /// Execute transcode with progress callback
    pub fn transcode<F>(&self, input: &Path, output: &Path, progress_callback: Option<F>) -> Result<()>
    where
        F: Fn(TranscodeProgress) + Send,
    {
        // Get input duration for progress calculation
        let media_info = self.probe(input)?;

        // Build FFmpeg command
        let mut cmd = Command::new(&self.executable);
        cmd.arg("-i").arg(input);

        // Add codec and preset arguments based on configuration
        self.add_codec_args(&mut cmd)?;

        // Output file
        cmd.arg("-y") // Overwrite output
            .arg(output)
            .stderr(Stdio::piped());

        // Execute command
        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RipperError::FFmpegNotFound
            } else {
                RipperError::FFmpegError(format!("Failed to spawn FFmpeg: {}", e))
            }
        })?;

        // Parse progress from stderr
        if let (Some(stderr), Some(callback)) = (child.stderr.take(), progress_callback) {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Some(progress) = parse_ffmpeg_progress(&line, media_info.duration) {
                        callback(progress);
                    }
                }
            }
        }

        // Wait for completion
        let status = child.wait()?;

        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            return Err(RipperError::FFmpegError(format!(
                "FFmpeg exited with code {}", exit_code
            )));
        }

        Ok(())
    }

    /// Add codec-specific arguments to FFmpeg command
    fn add_codec_args(&self, cmd: &mut Command) -> Result<()> {
        match self.preset {
            TranscodePreset::PassThrough => {
                cmd.args(&["-c", "copy"]);
            }
            TranscodePreset::Compatible => {
                // H.264 for maximum compatibility
                cmd.args(&["-c:v", "libx264", "-preset", "medium", "-crf", &self.crf.to_string()]);
                cmd.args(&["-c:a", "aac", "-b:a", "192k"]);
            }
            TranscodePreset::Balanced => {
                // H.265 balanced
                cmd.args(&["-c:v", "libx265", "-preset", "medium", "-crf", "20"]);
                cmd.args(&["-c:a", "aac", "-b:a", "192k"]);
            }
            TranscodePreset::HighQuality => {
                // H.265 high quality
                cmd.args(&["-c:v", "libx265", "-preset", "slow", "-crf", "18"]);
                cmd.args(&["-c:a", "aac", "-b:a", "256k"]);
            }
            TranscodePreset::Fast => {
                // H.265 fast
                cmd.args(&["-c:v", "libx265", "-preset", "fast", "-crf", "23"]);
                cmd.args(&["-c:a", "aac", "-b:a", "192k"]);
            }
            TranscodePreset::HardwareAuto => {
                // Use hardware acceleration if available
                match self.hardware_accel {
                    HardwareAccel::Nvidia => {
                        cmd.args(&["-c:v", "hevc_nvenc", "-preset", "p4", "-cq", &self.crf.to_string()]);
                    }
                    HardwareAccel::Intel => {
                        cmd.args(&["-c:v", "hevc_qsv", "-preset", "medium", "-global_quality", &self.crf.to_string()]);
                    }
                    HardwareAccel::Amd => {
                        cmd.args(&["-c:v", "hevc_amf", "-quality", "balanced", "-qp_i", &self.crf.to_string()]);
                    }
                    _ => {
                        // Fallback to software encoding
                        cmd.args(&["-c:v", "libx265", "-preset", "medium", "-crf", &self.crf.to_string()]);
                    }
                }
                cmd.args(&["-c:a", "aac", "-b:a", "192k"]);
            }
        }

        Ok(())
    }

    /// Check if FFmpeg executable is available
    pub fn check_available(&self) -> Result<bool> {
        let output = Command::new(&self.executable)
            .arg("-version")
            .output();

        match output {
            Ok(output) => Ok(output.status.success()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(RipperError::FFmpegError(format!("Failed to check FFmpeg: {}", e))),
        }
    }

    /// Detect available hardware encoders
    pub fn detect_hardware_accel(&self) -> Result<Vec<HardwareAccel>> {
        let output = Command::new(&self.executable)
            .args(&["-hide_banner", "-encoders"])
            .output()
            .map_err(|e| RipperError::FFmpegError(format!("Failed to detect encoders: {}", e)))?;

        let encoders = String::from_utf8_lossy(&output.stdout);
        let mut available = vec![HardwareAccel::None];

        if encoders.contains("hevc_nvenc") || encoders.contains("h264_nvenc") {
            available.push(HardwareAccel::Nvidia);
        }
        if encoders.contains("hevc_qsv") || encoders.contains("h264_qsv") {
            available.push(HardwareAccel::Intel);
        }
        if encoders.contains("hevc_amf") || encoders.contains("h264_amf") {
            available.push(HardwareAccel::Amd);
        }

        Ok(available)
    }

    /// Generate thumbnail from video
    pub fn generate_thumbnail(&self, input: &Path, output: &Path, timestamp_seconds: f64) -> Result<()> {
        let status = Command::new(&self.executable)
            .args(&[
                "-ss", &timestamp_seconds.to_string(),
                "-i", input.to_str().unwrap(),
                "-vframes", "1",
                "-q:v", "2",
                "-y",
                output.to_str().unwrap(),
            ])
            .status()
            .map_err(|e| RipperError::FFmpegError(format!("Failed to generate thumbnail: {}", e)))?;

        if !status.success() {
            return Err(RipperError::FFmpegError("Thumbnail generation failed".to_string()));
        }

        Ok(())
    }
}

/// Parse FFmpeg progress output
/// Format: frame=12345 fps=120 q=22.0 size=1234567kB time=00:30:00.00 bitrate=5000kbps speed=2.5x
fn parse_ffmpeg_progress(line: &str, total_duration: f64) -> Option<TranscodeProgress> {
    if !line.contains("frame=") {
        return None;
    }

    let mut frame = 0u64;
    let mut fps = 0.0f32;
    let mut time = String::new();
    let mut bitrate = String::new();
    let mut speed = 0.0f32;

    for part in line.split_whitespace() {
        if let Some(value) = part.strip_prefix("frame=") {
            frame = value.parse().unwrap_or(0);
        } else if let Some(value) = part.strip_prefix("fps=") {
            fps = value.parse().unwrap_or(0.0);
        } else if let Some(value) = part.strip_prefix("time=") {
            time = value.to_string();
        } else if let Some(value) = part.strip_prefix("bitrate=") {
            bitrate = value.to_string();
        } else if let Some(value) = part.strip_prefix("speed=") {
            speed = value.trim_end_matches('x').parse().unwrap_or(0.0);
        }
    }

    // Calculate percentage from time
    let percentage = if total_duration > 0.0 && !time.is_empty() {
        let time_seconds = parse_time_to_seconds(&time);
        ((time_seconds / total_duration) * 100.0).min(100.0) as f32
    } else {
        0.0
    };

    Some(TranscodeProgress {
        frame,
        fps,
        time,
        bitrate,
        speed,
        percentage,
    })
}

/// Parse time string (HH:MM:SS.MS) to seconds
fn parse_time_to_seconds(time_str: &str) -> f64 {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return 0.0;
    }

    let hours: f64 = parts[0].parse().unwrap_or(0.0);
    let minutes: f64 = parts[1].parse().unwrap_or(0.0);
    let seconds: f64 = parts[2].parse().unwrap_or(0.0);

    hours * 3600.0 + minutes * 60.0 + seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcoder_creation() {
        let transcoder = FFmpegTranscoder::new("ffmpeg");
        assert_eq!(transcoder.crf, 20);
        assert_eq!(transcoder.preset, TranscodePreset::Balanced);
        assert_eq!(transcoder.hardware_accel, HardwareAccel::None);
    }

    #[test]
    fn test_transcoder_preset_chain() {
        let transcoder = FFmpegTranscoder::new("ffmpeg")
            .with_preset(TranscodePreset::HighQuality)
            .with_crf(18)
            .with_hardware_accel(HardwareAccel::Nvidia);

        assert_eq!(transcoder.preset, TranscodePreset::HighQuality);
        assert_eq!(transcoder.crf, 18);
        assert_eq!(transcoder.hardware_accel, HardwareAccel::Nvidia);
    }

    #[test]
    fn test_crf_limits() {
        let transcoder = FFmpegTranscoder::new("ffmpeg").with_crf(100);
        assert_eq!(transcoder.crf, 51); // Should be capped at 51
    }

    #[test]
    fn test_parse_time_to_seconds() {
        assert_eq!(parse_time_to_seconds("00:00:10.00"), 10.0);
        assert_eq!(parse_time_to_seconds("00:01:30.50"), 90.5);
        assert_eq!(parse_time_to_seconds("01:30:45.25"), 5445.25);
        assert_eq!(parse_time_to_seconds("invalid"), 0.0);
    }

    #[test]
    fn test_parse_ffmpeg_progress() {
        let line = "frame=12345 fps=120.5 q=22.0 size=1234567kB time=00:30:00.00 bitrate=5000kbps speed=2.5x";
        let progress = parse_ffmpeg_progress(line, 3600.0).unwrap();

        assert_eq!(progress.frame, 12345);
        assert_eq!(progress.fps, 120.5);
        assert_eq!(progress.time, "00:30:00.00");
        assert_eq!(progress.bitrate, "5000kbps");
        assert_eq!(progress.speed, 2.5);
        assert!((progress.percentage - 50.0).abs() < 1.0); // Should be ~50% of 3600s
    }

    #[test]
    fn test_parse_ffmpeg_progress_no_frame() {
        let line = "Some other output without frame info";
        assert!(parse_ffmpeg_progress(line, 3600.0).is_none());
    }

    #[test]
    fn test_hardware_accel_display() {
        assert_eq!(format!("{}", HardwareAccel::None), "None");
        assert_eq!(format!("{}", HardwareAccel::Nvidia), "NVENC");
        assert_eq!(format!("{}", HardwareAccel::Intel), "QuickSync");
        assert_eq!(format!("{}", HardwareAccel::Amd), "AMF");
        assert_eq!(format!("{}", HardwareAccel::Auto), "Auto");
    }

    #[test]
    fn test_transcode_preset_equality() {
        assert_eq!(TranscodePreset::Balanced, TranscodePreset::Balanced);
        assert_ne!(TranscodePreset::Balanced, TranscodePreset::Fast);
    }

    #[test]
    fn test_hardware_accel_equality() {
        assert_eq!(HardwareAccel::Nvidia, HardwareAccel::Nvidia);
        assert_ne!(HardwareAccel::Nvidia, HardwareAccel::Intel);
    }

    #[test]
    fn test_progress_percentage_calculation() {
        // Test at 25% of 1 hour (900s of 3600s)
        let line = "frame=1000 fps=30 time=00:15:00.00 bitrate=5000kbps speed=1.0x";
        let progress = parse_ffmpeg_progress(line, 3600.0).unwrap();
        assert!((progress.percentage - 25.0).abs() < 1.0);
    }

    #[test]
    fn test_progress_percentage_max() {
        // Should never exceed 100%
        let line = "frame=1000 fps=30 time=02:00:00.00 bitrate=5000kbps speed=1.0x";
        let progress = parse_ffmpeg_progress(line, 3600.0).unwrap();
        assert!(progress.percentage <= 100.0);
    }

    #[test]
    fn test_media_info_structure() {
        let info = MediaInfo {
            duration: 3600.0,
            video_codec: "h264".to_string(),
            width: 1920,
            height: 1080,
            bitrate: 5000000,
            file_size: 2147483648,
        };

        assert_eq!(info.duration, 3600.0);
        assert_eq!(info.video_codec, "h264");
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 1080);
    }
}
