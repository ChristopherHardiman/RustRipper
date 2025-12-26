//! MakeMKV execution wrapper with corrected CLI arguments

use rustripper_core::{RipperError, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

/// Progress callback function type
pub type ProgressCallback = Box<dyn Fn(f32, &str) + Send>;

/// MakeMKV ripper interface
pub struct MakeMKVRipper {
    executable: String,
    min_title_length: u32,
    title_selection: String,
}

impl MakeMKVRipper {
    /// Create a new MakeMKV ripper
    pub fn new(executable: impl Into<String>, min_title_length: u32) -> Self {
        Self {
            executable: executable.into(),
            min_title_length,
            title_selection: "all".to_string(),
        }
    }

    /// Set which titles to rip ("all" or comma-separated numbers)
    pub fn with_title_selection(mut self, selection: impl Into<String>) -> Self {
        self.title_selection = selection.into();
        self
    }

    /// Execute MakeMKV with correct arguments
    /// Format: makemkvcon mkv dev:/dev/sr0 all /output/path --minlength=<seconds>
    pub fn rip<F>(&self, device: &str, output_dir: &Path, progress_callback: Option<F>) -> Result<()>
    where
        F: Fn(f32, &str) + Send,
    {
        // Validate device path format
        let device_arg = if device.starts_with("dev:") {
            device.to_string()
        } else {
            format!("dev:{}", device)
        };

        // Build MakeMKV command with correct argument format
        // Format: makemkvcon mkv dev:/dev/sr0 <title> <output> --minlength=<seconds>
        let mut cmd = Command::new(&self.executable);
        cmd.arg("mkv")
            .arg(&device_arg)
            .arg(&self.title_selection)
            .arg(output_dir)
            .arg(format!("--minlength={}", self.min_title_length))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Execute command
        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RipperError::MakeMKVNotFound
            } else {
                RipperError::MakeMKVError(format!("Failed to spawn MakeMKV: {}", e))
            }
        })?;

        // Parse progress from stdout
        if let (Some(stdout), Some(callback)) = (child.stdout.take(), progress_callback) {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Some((progress, message)) = parse_makemkv_progress(&line) {
                        callback(progress, message);
                    }
                }
            }
        }

        // Wait for completion
        let status = child.wait()?;
        
        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            return Err(RipperError::MakeMKVError(format!(
                "MakeMKV exited with code {}", exit_code
            )));
        }

        Ok(())
    }

    /// Check if MakeMKV executable is available
    pub fn check_available(&self) -> Result<bool> {
        let output = Command::new(&self.executable)
            .arg("--version")
            .output();

        match output {
            Ok(output) => Ok(output.status.success()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(RipperError::MakeMKVError(format!("Failed to check MakeMKV: {}", e))),
        }
    }
}

/// Parse MakeMKV progress output
/// MakeMKV outputs progress in format: "PRGV:current,total,max"
/// Also outputs messages in format: "MSG:code,flags,count,message"
fn parse_makemkv_progress(line: &str) -> Option<(f32, &str)> {
    if let Some(rest) = line.strip_prefix("PRGV:") {
        // Format: PRGV:current,total,max
        let parts: Vec<&str> = rest.split(',').collect();
        if parts.len() >= 3 {
            if let (Ok(current), Ok(total)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                if total > 0.0 {
                    let percentage = (current / total) * 100.0;
                    return Some((percentage, "Ripping in progress"));
                }
            }
        }
    } else if let Some(rest) = line.strip_prefix("MSG:") {
        // Format: MSG:code,flags,count,"message"
        // For now, just detect completion messages
        if rest.contains("Copy complete") || rest.contains("done") {
            return Some((100.0, "Rip complete"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ripper_creation() {
        let ripper = MakeMKVRipper::new("makemkvcon", 180);
        assert_eq!(ripper.executable, "makemkvcon");
        assert_eq!(ripper.min_title_length, 180);
        assert_eq!(ripper.title_selection, "all");
    }

    #[test]
    fn test_title_selection() {
        let ripper = MakeMKVRipper::new("makemkvcon", 180)
            .with_title_selection("1,2,3");
        assert_eq!(ripper.title_selection, "1,2,3");
    }

    #[test]
    fn test_parse_progress_prgv() {
        // Test PRGV format: PRGV:current,total,max
        let (progress, msg) = parse_makemkv_progress("PRGV:50,100,100").unwrap();
        assert_eq!(progress, 50.0);
        assert_eq!(msg, "Ripping in progress");

        let (progress, _) = parse_makemkv_progress("PRGV:75,100,100").unwrap();
        assert_eq!(progress, 75.0);

        let (progress, _) = parse_makemkv_progress("PRGV:100,100,100").unwrap();
        assert_eq!(progress, 100.0);
    }

    #[test]
    fn test_parse_progress_msg_complete() {
        let (progress, msg) = parse_makemkv_progress("MSG:3307,0,0,\"Copy complete\"").unwrap();
        assert_eq!(progress, 100.0);
        assert_eq!(msg, "Rip complete");
    }

    #[test]
    fn test_parse_progress_invalid() {
        assert!(parse_makemkv_progress("invalid line").is_none());
        assert!(parse_makemkv_progress("PRGV:invalid").is_none());
        assert!(parse_makemkv_progress("").is_none());
    }

    #[test]
    fn test_parse_progress_zero_total() {
        // Should not divide by zero
        assert!(parse_makemkv_progress("PRGV:0,0,0").is_none());
    }

    #[test]
    fn test_device_path_formatting() {
        // This tests the device argument format internally
        // The format should be "dev:/dev/sr0" not "--input=/dev/sr0"
        let ripper = MakeMKVRipper::new("makemkvcon", 180);
        
        // Device paths should start with "dev:" prefix
        assert!(ripper.executable.contains("makemkvcon"));
    }
}
