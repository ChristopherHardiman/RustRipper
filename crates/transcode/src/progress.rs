//! Progress information parsed from FFmpeg output

/// Progress information from FFmpeg execution
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Current frame number
    pub frame: Option<u64>,
    /// Current frames per second
    pub fps: Option<f32>,
    /// Current quality metric
    pub q: Option<f32>,
    /// Size of output so far in KB
    pub size_kb: Option<u64>,
    /// Current timestamp in the video
    pub time: Option<String>,
    /// Current bitrate
    pub bitrate: Option<String>,
    /// Speed multiplier (e.g., 2.5x)
    pub speed: Option<f32>,
}

impl ProgressInfo {
    /// Parse FFmpeg stderr line for progress information
    /// Expected format: "frame=12345 fps=120 q=22.0 size=1234567kB time=00:30:00.00 bitrate=5000kbps speed=2.5x"
    pub fn parse_ffmpeg_line(line: &str) -> Option<Self> {
        // TODO: Implement parsing with regex
        None
    }

    /// Calculate percentage progress based on duration and current time
    pub fn calculate_percentage(&self, total_seconds: f32) -> Option<f32> {
        // TODO: Parse time string and calculate percentage
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_creation() {
        let progress = ProgressInfo {
            frame: Some(1000),
            fps: Some(30.0),
            q: None,
            size_kb: None,
            time: Some("00:00:33.33".to_string()),
            bitrate: None,
            speed: Some(1.0),
        };
        assert_eq!(progress.frame, Some(1000));
    }
}
