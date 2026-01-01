# MasterRustRipper

A modern, containerized automation suite for ripping and transcoding DVDs and Blu-ray discs, built with Rust and accessible through a web interface.

## Overview

MasterRustRipper automates the entire disc ripping workflow: from detecting inserted discs to fetching metadata, ripping content, transcoding videos, and organizing your media library. Monitor and control everything through a responsive web dashboard accessible from any device on your network.

## Key Features

- **Automatic Disc Detection** - Monitors your optical drive and starts ripping automatically when a disc is inserted
- **Smart Metadata Lookup** - Fetches movie/TV/anime information from multiple sources (TMDb, OMDb, AniList, TheTVDB)
- **Intelligent Transcoding** - Re-encodes videos with H.265/HEVC for 50-60% size reduction while maintaining quality
- **Hardware Acceleration** - Supports NVIDIA NVENC, Intel QuickSync, and AMD VCE for faster encoding
- **Web-Based Interface** - Real-time progress monitoring and job management from any browser
- **Containerized Architecture** - Isolated dependencies using Podman for clean, reproducible deployments
- **Duplicate Detection** - Prevents re-ripping discs you've already processed
- **Job Queue Management** - Queue multiple discs and manage priorities

## Technology Stack

- **Backend**: Rust (Axum web framework)
- **Frontend**: Modern web framework (SvelteKit/React/Vue)
- **Database**: SQLite for job history and configuration
- **Containers**: Podman with podman-compose
- **Ripping**: MakeMKV
- **Transcoding**: FFmpeg with hardware acceleration support
- **Decryption**: libdvdcss, libaacs, libbdplus

## Project Scope

### Supported Media Types
- DVD Video (encrypted and unencrypted)
- Blu-ray Discs (with AACS/BD+ decryption)
- Movies, TV Shows, and Anime

### Workflow Pipeline
1. **Detect** → Monitor optical drive for disc insertion
2. **Identify** → Fetch metadata (title, year, genre, poster art)
3. **Rip** → Extract video to MKV format using MakeMKV
4. **Transcode** → Re-encode with H.265 for smaller file sizes
5. **Organize** → Save to structured directories with proper naming

### Architecture
- **Modular Library Crates** - Reusable components for disc detection, metadata, ripping, and transcoding
- **REST API + WebSockets** - Backend server for job management and real-time updates
- **Worker Containers** - Isolated MakeMKV and FFmpeg execution environments
- **Web Dashboard** - Responsive UI for monitoring jobs, browsing history, and configuration

## Current Status

### ✅ Phase 1: Core Libraries (100% Complete)

All foundational libraries are implemented, tested, and production-ready:

- **rustripper-core** - Error handling, domain types, configuration (5 tests)
- **rustripper-disc** - Optical drive detection via blkid (7 tests)
- **rustripper-ripper** - MakeMKV wrapper with progress tracking (8 tests)
- **rustripper-metadata** - TMDb, AniList, and OMDb providers (35 tests)
- **rustripper-transcode** - FFmpeg wrapper with hardware acceleration (13 tests)
- **rustripper-storage** - Database operations stub (1 test)

**Total: 68 passing unit tests**

### 🚧 Phase 2: CLI Binary (Next)

Command-line interface for testing with real hardware.

### 🔮 Future Phases

- **Phase 3:** REST API backend with Axum
- **Phase 4:** Containerization with Podman
- **Phase 5:** Web UI with Svelte

## Getting Started

### Prerequisites

**System Dependencies:**
- Rust (1.75+)
- Podman and podman-compose
- MakeMKV (with libdvdcss for DVD decryption)
- FFmpeg (with hardware acceleration support optional)
- Blu-ray decryption: libaacs, libbdplus, and KEYDB.cfg

**Hardware:**
- Optical drive (DVD or Blu-ray)
- Sufficient disk space for ripped media
- (Optional) NVIDIA/Intel/AMD GPU for hardware-accelerated transcoding

### Installation

*Installation instructions will be provided once Phase 2 CLI is complete.*

### Quick Start

*Usage instructions will be provided once the CLI and web interface are implemented.*

## Documentation

- [Implementation Plan](implementation_plan.md) - Detailed development roadmap and architecture
- [Code Schema](code_schema.md) - Configuration examples, database schema, and command references
- [Current Status](current_status.md) - Project progress and known issues

## Contributing

This project is in early development. Phase 1 (core libraries) is complete. Contributions are welcome as we move forward with Phase 2 (CLI) and beyond. Please check the implementation plan for planned features and the current status document for areas that need work.

## License

See [LICENSE](LICENSE) file for details.

## Acknowledgments

- **MakeMKV** - DVD/Blu-ray ripping engine
- **FFmpeg** - Video transcoding and analysis
- **TMDb, OMDb, AniList, TheTVDB** - Metadata providers

---

**Note**: This project is for personal media backup purposes. Users are responsible for complying with copyright laws in their jurisdiction.
