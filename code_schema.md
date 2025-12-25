# RustRipper Code Examples and Schema

This document contains code examples, installation commands, and schemas referenced in the implementation plan.

---

## Table of Contents

1. [System Dependencies Installation](#system-dependencies-installation)
2. [FFmpeg Commands](#ffmpeg-commands)
3. [Database Schema](#database-schema)
4. [Configuration File Schema](#configuration-file-schema)

---

## System Dependencies Installation

### Fedora: libdvdcss Installation

```bash
# Step 1: Enable RPM Fusion Free repository
sudo dnf install https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm

# Step 2: Enable RPM Fusion Non-Free repository (optional, but recommended)
sudo dnf install https://download1.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm

# Step 3: Install libdvdcss
sudo dnf install libdvdcss
```

### KEYDB.cfg Download

```bash
# Create key directories
mkdir -p ~/.config/aacs
mkdir -p ~/.config/bdplus

# Download KEYDB.cfg (required for libaacs)
wget -O ~/.config/aacs/KEYDB.cfg "http://fvonline-db.bplaced.net/fv_download.php?lang=eng"
```

---

## FFmpeg Commands

### H.265 Encoding (Software)

```bash
ffmpeg -i input.mkv -c:v libx265 -crf 20 -preset medium -c:a aac -b:a 256k output.mkv
```

### Hardware Acceleration - NVIDIA

```bash
ffmpeg -i input.mkv -c:v hevc_nvenc -preset p4 -cq 20 -c:a copy output.mkv
```

### Hardware Acceleration - Intel QuickSync

```bash
ffmpeg -i input.mkv -c:v hevc_qsv -preset medium -global_quality 22 -c:a copy output.mkv
```

### Hardware Acceleration - AMD

```bash
ffmpeg -i input.mkv -c:v hevc_amf -quality quality -c:a copy output.mkv
```

### Analyze Media File

```bash
ffprobe -v quiet -print_format json -show_format -show_streams movie.mkv
```

---

## Database Schema

### SQLite Schema

**File:** `backend/src/db/schema.sql`

```sql
-- Jobs table: Active and historical job records
CREATE TABLE jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    disc_label TEXT NOT NULL,
    title TEXT,
    year INTEGER,
    status TEXT NOT NULL,
    progress REAL DEFAULT 0.0,
    stage TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    error TEXT
);

-- Rips table: Completed rip statistics
CREATE TABLE rips (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id INTEGER REFERENCES jobs(id),
    disc_id TEXT UNIQUE,
    output_path TEXT NOT NULL,
    original_size INTEGER,
    final_size INTEGER,
    duration_seconds INTEGER,
    completed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Config table: Dynamic configuration storage
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX idx_jobs_status ON jobs(status);
CREATE INDEX idx_rips_disc_id ON rips(disc_id);
CREATE INDEX idx_jobs_created_at ON jobs(created_at);
```

---

## Configuration File Schema

### TOML Configuration

**Location:** `~/.config/masterrustripper/config.toml`

```toml
#
# MasterRustRipper Configuration
#

[general]
output_path = "/home/user/Videos/Ripped"
disc_device = "/dev/sr0"
auto_eject = true
check_duplicates = true
auto_rip_on_insert = false

[makemkv]
executable = "makemkvcon"  # or full path to binary
min_title_length = 120      # seconds, ignore titles shorter than this
title_selection = "all"     # "all" or specific title number

[ffmpeg]
enabled = true
executable = "ffmpeg"
ffprobe_executable = "ffprobe"

# Video encoding
codec = "libx265"           # libx264, libx265, hevc_nvenc, hevc_qsv, hevc_amf
preset = "medium"           # ultrafast, fast, medium, slow, veryslow
crf = 20                    # 0-51, lower = better quality (18-23 recommended)
hardware_accel = "auto"     # auto, nvidia, intel, amd, none

# Audio encoding
audio_codec = "aac"
audio_bitrate = "256k"
keep_original_audio = false # true = copy audio streams unchanged

# File management
delete_original = false
min_size_for_transcode = 1073741824  # 1 GB - don't transcode smaller files

[metadata]
# Priority order for lookups (first match wins)
sources = ["tmdb", "anilist", "omdb", "tvdb"]

[metadata.tmdb]
api_key = ""
language = "en-US"
include_adult = false

[metadata.omdb]
api_key = ""

[metadata.anilist]
# No API key required
preferred_title = "english"  # romaji, english, native

[metadata.tvdb]
api_key = ""

[output]
# Available variables: {title}, {year}, {season}, {episode}
movie_template = "{title} ({year})"
tv_template = "{title}/Season {season}"
anime_template = "{title} ({year})"
generate_thumbnails = true
download_artwork = true

[notifications]
desktop_enabled = true
sound_enabled = false

[history]
enabled = true
database_path = "~/.local/share/masterrustripper/history.db"
```

---

## Backend Project Structure

```
backend/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── api/
    │   ├── mod.rs
    │   ├── routes.rs      # REST endpoints
    │   ├── websocket.rs   # WebSocket handler
    │   └── middleware.rs  # CORS, logging
    ├── jobs/
    │   ├── mod.rs
    │   ├── queue.rs       # Job queue management
    │   └── executor.rs    # Job execution logic
    ├── disc/
    │   └── watcher.rs     # Background disc monitoring
    └── db/
        ├── mod.rs
        ├── schema.sql
        └── queries.rs
```

---

## Container Commands

### Build Backend Container

```bash
podman build -t rustripper-backend ./backend
```

### Build MakeMKV Container

```bash
podman build -t rustripper-makemkv ./containers/makemkv
```

### Start All Services

```bash
podman-compose up -d
```

### Stop All Services

```bash
podman-compose down
```

### View Container Logs

```bash
podman logs -f rustripper-backend
podman logs -f rustripper-makemkv
podman logs -f rustripper-ffmpeg
```

### Execute Command in Container

```bash
# Rip disc
podman exec rustripper-makemkv makemkvcon mkv dev:/dev/sr0 all /mnt/rips

# Transcode video
podman exec rustripper-ffmpeg ffmpeg -i /mnt/rips/input.mkv -c:v libx265 -crf 20 /mnt/rips/output.mkv
```

---

## Build and Deployment

### Workspace Build

```bash
# Build all projects in workspace
cargo build --workspace

# Build release versions
cargo build --workspace --release

# Run specific binary
cargo run -p backend
cargo run -p cli -- watch
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p disc
cargo test -p metadata

# Run with logging
RUST_LOG=debug cargo test --workspace
```

---

## Example Output Directory Structure

```
/home/user/Videos/Ripped/
├── Inception (2010)/
│   ├── Inception (2010).mkv
│   ├── poster.jpg
│   └── thumbnail.jpg
├── Cowboy Bebop (1998)/
│   ├── Cowboy Bebop (1998) - Disc 1.mkv
│   ├── Cowboy Bebop (1998) - Disc 2.mkv
│   ├── poster.jpg
│   └── thumbnail.jpg
└── Breaking Bad/
    ├── Season 1/
    │   ├── Breaking Bad S01E01.mkv
    │   ├── Breaking Bad S01E02.mkv
    │   └── ...
    └── Season 2/
        ├── Breaking Bad S02E01.mkv
        └── ...
```
