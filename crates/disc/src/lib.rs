//! Disc detection and monitoring for optical drives

pub mod watcher;

pub use watcher::{DiscWatcher, detect_disc, parse_blkid_output};
