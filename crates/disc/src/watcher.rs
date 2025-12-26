//! Disc watcher for detecting optical disc insertion/ejection

use rustripper_core::{DiscInfo, DiscType, Result};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Watches for disc insertion and ejection events
pub struct DiscWatcher {
    device_path: String,
    poll_interval: Duration,
    last_disc_state: Option<DiscInfo>,
}

impl DiscWatcher {
    /// Create a new disc watcher
    pub fn new(device_path: impl Into<String>) -> Self {
        Self {
            device_path: device_path.into(),
            poll_interval: Duration::from_secs(2),
            last_disc_state: None,
        }
    }

    /// Set the polling interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Poll the disc device and detect changes
    pub fn poll(&mut self) -> Result<Option<DiscInfo>> {
        let current_state = detect_disc(&self.device_path)?;
        
        // Compare with last state to detect changes
        let changed = match (&self.last_disc_state, &current_state) {
            (None, Some(_)) => true,  // Disc inserted
            (Some(_), None) => true,  // Disc ejected
            (Some(old), Some(new)) => old.label != new.label, // Different disc
            (None, None) => false,    // No change
        };
        
        if changed {
            self.last_disc_state = current_state.clone();
            Ok(current_state)
        } else {
            Ok(None)
        }
    }

    /// Check if a device path is a valid optical drive
    pub fn is_valid_device(path: &str) -> bool {
        Path::new(path).exists()
    }
}

/// Detect disc information using blkid
pub fn detect_disc(device_path: &str) -> Result<Option<DiscInfo>> {
    // Check if device exists first
    if !Path::new(device_path).exists() {
        return Ok(None);
    }

    let output = Command::new("blkid")
        .arg("-o")
        .arg("export")
        .arg(device_path)
        .output()?;

    if !output.status.success() {
        // No disc present or device not readable
        return Ok(None);
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    parse_blkid_output(&output_str, device_path)
}

/// Parse blkid export format output
pub fn parse_blkid_output(output: &str, device_path: &str) -> Result<Option<DiscInfo>> {
    let mut disc_type_str: Option<String> = None;
    let mut disc_label: Option<String> = None;

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("TYPE=") {
            disc_type_str = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("LABEL=") {
            disc_label = Some(value.to_string());
        }
    }

    // If we got any information, create DiscInfo
    if disc_type_str.is_some() || disc_label.is_some() {
        let disc_type = disc_type_str
            .as_deref()
            .and_then(map_disc_type)
            .unwrap_or(DiscType::Unknown);

        let label = disc_label.unwrap_or_else(|| "UNTITLED".to_string());

        Ok(Some(DiscInfo {
            device: device_path.to_string(),
            label,
            disc_type,
            readable: true,
        }))
    } else {
        Ok(None)
    }
}

/// Map filesystem type string to DiscType
fn map_disc_type(type_str: &str) -> Option<DiscType> {
    match type_str.to_lowercase().as_str() {
        "udf" | "iso9660" => Some(DiscType::DVD),
        "udf_bd" => Some(DiscType::BluRay),
        _ => Some(DiscType::Unknown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disc_watcher_creation() {
        let watcher = DiscWatcher::new("/dev/sr0");
        assert_eq!(watcher.device_path, "/dev/sr0");
    }

    #[test]
    fn test_parse_blkid_dvd() {
        let output = "DEVNAME=/dev/sr0\nLABEL=MY_MOVIE_DISC\nTYPE=udf\n";
        let result = parse_blkid_output(output, "/dev/sr0").unwrap();
        
        assert!(result.is_some());
        let disc = result.unwrap();
        assert_eq!(disc.label, "MY_MOVIE_DISC");
        assert_eq!(disc.disc_type, DiscType::DVD);
        assert_eq!(disc.device, "/dev/sr0");
        assert!(disc.readable);
    }

    #[test]
    fn test_parse_blkid_bluray() {
        let output = "DEVNAME=/dev/sr0\nLABEL=AVATAR_2009\nTYPE=udf_bd\n";
        let result = parse_blkid_output(output, "/dev/sr0").unwrap();
        
        assert!(result.is_some());
        let disc = result.unwrap();
        assert_eq!(disc.label, "AVATAR_2009");
        assert_eq!(disc.disc_type, DiscType::BluRay);
    }

    #[test]
    fn test_parse_blkid_no_label() {
        let output = "DEVNAME=/dev/sr0\nTYPE=iso9660\n";
        let result = parse_blkid_output(output, "/dev/sr0").unwrap();
        
        assert!(result.is_some());
        let disc = result.unwrap();
        assert_eq!(disc.label, "UNTITLED");
        assert_eq!(disc.disc_type, DiscType::DVD);
    }

    #[test]
    fn test_parse_blkid_empty() {
        let output = "";
        let result = parse_blkid_output(output, "/dev/sr0").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_blkid_unknown_type() {
        let output = "DEVNAME=/dev/sr0\nLABEL=MUSIC_CD\nTYPE=iso9660\n";
        let result = parse_blkid_output(output, "/dev/sr0").unwrap();
        
        assert!(result.is_some());
        let disc = result.unwrap();
        assert_eq!(disc.label, "MUSIC_CD");
        assert_eq!(disc.disc_type, DiscType::DVD); // iso9660 maps to DVD
    }

    #[test]
    fn test_parse_blkid_with_underscores() {
        let output = "DEVNAME=/dev/sr0\nLABEL=INCEPTION_2010\nTYPE=udf\n";
        let result = parse_blkid_output(output, "/dev/sr0").unwrap();
        
        assert!(result.is_some());
        let disc = result.unwrap();
        assert_eq!(disc.label, "INCEPTION_2010");
    }
}
