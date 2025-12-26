//! FFmpeg transcoding wrapper with progress parsing and hardware acceleration support

pub mod encoder;
pub mod progress;

pub use encoder::FFmpegTranscoder;
pub use progress::ProgressInfo;
