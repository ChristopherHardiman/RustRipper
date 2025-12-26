# MasterRustRipper

A containerized Rust-based DVD/Blu-ray ripping automation suite with metadata enrichment and intelligent transcoding.

## Overview

MasterRustRipper is designed to automate the entire disc ripping workflow:
1. **Detect** optical discs automatically
2. **Rip** with MakeMKV 
3. **Enrich** with metadata from TMDb, AniList, and OMDb
4. **Transcode** with FFmpeg using intelligent presets
5. **Organize** your media library with proper naming and structure

## Project Status

### ✅ Phase 1: Core Libraries (100% Complete)

All foundational libraries are implemented, tested, and production-ready:

- **rustripper-core** - Error handling, domain types, configuration (5 tests)
- **rustripper-disc** - Optical drive detection via blkid (7 tests)
- **rustripper-ripper** - MakeMKV wrapper with progress tracking (7 tests)
- **rustripper-metadata** - TMDb, AniList, and OMDb providers (35 tests)
- **rustripper-transcode** - FFmpeg wrapper with hardware acceleration (13 tests)
- **rustripper-storage** - Database operations stub (1 test)

**Total: 68 passing unit tests**

### 🚧 Phase 2: CLI Binary (Planned)

Command-line interface for testing with real hardware (not yet implemented).

### 🔮 Future Phases

- **Phase 3:** REST API backend with Axum
- **Phase 4:** Containerization with Podman
- **Phase 5:** Web UI with Svelte

## Current Usage (Library API)

Since the CLI is not yet implemented, you can use the libraries directly in your Rust code:

### Running Tests

```bash
# Run all tests
cargo test --workspace --lib

# Run specific crate tests
cargo test -p rustripper-disc
cargo test -p rustripper-metadata
cargo test -p rustripper-ripper
cargo test -p rustripper-transcode

# Build the workspace
cargo build --workspace

# Check for errors
cargo check --workspace
```

### Example: Disc Detection

```rust
use rustripper_disc::DiscWatcher;

let mut watcher = DiscWatcher::new("/dev/sr0");
if let Ok(Some(disc)) = watcher.poll() {
    println!("Detected: {} ({})", disc.label, disc.disc_type);
}
```

### Example: Metadata Lookup

```rust
use rustripper_metadata::MetadataAggregator;

let aggregator = MetadataAggregator::new()
    .with_tmdb("YOUR_TMDB_API_KEY")
    .with_omdb("YOUR_OMDB_API_KEY");

// Automatic label sanitization and provider priority
let results = aggregator
    .search_with_type_detection("The_Matrix_1999")
    .await?;
```

### Example: Complete Workflow

See [examples/disc_and_metadata.rs](examples/disc_and_metadata.rs) for a full integration example:

```bash
cargo run --example disc_and_metadata
```

## Planned CLI Commands (Phase 2)

Once Phase 2 is implemented, the following commands will be available:

### `rustripper watch`
Monitor optical drive and auto-rip on disc insertion
```bash
rustripper watch --device /dev/sr0
```

### `rustripper rip`
Manually rip current disc
```bash
# Rip all titles
rustripper rip --device /dev/sr0 --output /media/rips

# Rip specific title
rustripper rip --device /dev/sr0 --title 0 --output /media/rips

# Set minimum title length (default: 180 seconds)
rustripper rip --device /dev/sr0 --min-length 300 --output /media/rips
```

### `rustripper search`
Search metadata providers
```bash
# Search all providers
rustripper search "Inception"

# Search with year filter
rustripper search "The Matrix" --year 1999

# Search specific provider
rustripper search "Cowboy Bebop" --provider anilist
```

### `rustripper transcode`
Transcode video files
```bash
# Use balanced preset (default)
rustripper transcode input.mkv output.mkv

# Use high quality preset
rustripper transcode input.mkv output.mkv --preset high-quality

# Use hardware acceleration (auto-detect)
rustripper transcode input.mkv output.mkv --preset hardware-auto

# Custom CRF value
rustripper transcode input.mkv output.mkv --crf 18

# Generate thumbnail
rustripper transcode input.mkv --thumbnail thumb.jpg --time 300
```

### `rustripper config`
View or edit configuration
```bash
# Show current config
rustripper config show

# Edit config file
rustripper config edit

# Set specific value
rustripper config set output_directory /media/library
```

### `rustripper history`
View ripping history (Phase 3)
```bash
# List all jobs
rustripper history list

# Show job details
rustripper history show <job-id>

# Clear history
rustripper history clear
```

## Architecture

### Workspace Structure
```
RustRipper/
├── Cargo.toml           # Workspace configuration
├── crates/
│   ├── core/            # Shared types, errors, config
│   ├── disc/            # Optical drive detection
│   ├── metadata/        # TMDb, AniList, OMDb clients
│   ├── ripper/          # MakeMKV wrapper
│   ├── transcode/       # FFmpeg wrapper
│   └── storage/         # Database operations (stub)
└── examples/            # Usage examples
```

### Key Features

**Disc Detection:**
- Polls optical drive using blkid
- Detects DVD and Blu-ray discs
- State change detection (insert/eject/swap)

**MakeMKV Integration:**
- Correct CLI format: `dev:/dev/sr0`
- Real-time progress parsing
- Configurable title selection
- Minimum title length filtering

**Metadata Providers:**
- **TMDb**: Movies and TV shows with poster URLs
- **AniList**: Anime (no API key required, GraphQL)
- **OMDb**: Basic movie/TV info with IMDb IDs
- **Aggregator**: Multi-provider search with fallback
- Automatic disc label sanitization (underscores, years, disc numbers)

**FFmpeg Transcoding:**
- 6 presets: Balanced, HighQuality, Fast, Compatible, HardwareAuto, PassThrough
- Hardware acceleration: NVENC (Nvidia), QuickSync (Intel), AMF (AMD)
- Real-time progress callbacks
- Media analysis via ffprobe
- Thumbnail generation

## Requirements

### Runtime Dependencies
- **blkid** - Disc detection (usually pre-installed on Linux)
- **MakeMKV** - DVD/Blu-ray ripping
- **FFmpeg** - Video transcoding
- **ffprobe** - Media analysis (comes with FFmpeg)

### API Keys (Optional)
- **TMDb API Key** - For movie/TV metadata (free at https://www.themoviedb.org/settings/api)
- **OMDb API Key** - For additional metadata (free at http://www.omdbapi.com/apikey.aspx)
- **AniList** - No API key required!

### Installation (Debian/Ubuntu)
```bash
# Install MakeMKV
sudo add-apt-repository ppa:heyarje/makemkv-beta
sudo apt update
sudo apt install makemkv-bin makemkv-oss

# Install FFmpeg
sudo apt install ffmpeg

# blkid is usually pre-installed
which blkid  # Should return /usr/bin/blkid or similar
```

## Development

### Build
```bash
cargo build --workspace
```

### Test
```bash
cargo test --workspace --lib
```

### Run Example
```bash
cargo run --example disc_and_metadata
```

### Documentation
```bash
cargo doc --workspace --open
```

## Configuration

Configuration will be stored in `~/.config/rustripper/config.toml` (Phase 2+):

```toml
[paths]
output_directory = "/media/library"
temp_directory = "/tmp/rustripper"

[makemkv]
binary_path = "makemkvcon"
min_title_length_seconds = 180

[ffmpeg]
binary_path = "ffmpeg"
default_preset = "Balanced"
default_crf = 20

[metadata]
tmdb_api_key = "your_key_here"
omdb_api_key = "your_key_here"
prefer_provider = "tmdb"  # tmdb, anilist, omdb
```

## Contributing

This project is in early development. Phase 1 (core libraries) is complete. Next steps:
1. Implement Phase 2 CLI for hardware testing
2. Test with real optical drives and discs
3. Validate MakeMKV and FFmpeg integrations
4. Refine metadata matching algorithms

## License

[Add your license here]

## Acknowledgments

- **MakeMKV** - DVD/Blu-ray decryption and ripping
- **FFmpeg** - Video transcoding
- **TMDb** - The Movie Database API
- **AniList** - Anime metadata API
- **OMDb** - Open Movie Database
