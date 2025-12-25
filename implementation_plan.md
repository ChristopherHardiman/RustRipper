# MasterRustRipper Implementation Plan

## Executive Summary

MasterRustRipper is a **containerized** Rust-based automation suite for ripping DVDs and Blu-ray discs with a **web-based interface**. The project uses Podman containers to isolate dependencies and provide a modern web UI accessible from any device on the network. The workflow consists of: disc detection → metadata lookup → ripping → transcoding → organization, all managed through a web dashboard with real-time progress updates.

---

## Table of Contents

1. [Current State Assessment](#current-state-assessment)
2. [Target Architecture](#target-architecture)
3. [External Dependencies](#external-dependencies)
4. [Implementation Phases](#implementation-phases)
5. [Configuration Schema](#configuration-schema)
6. [Implementation Schedule](#implementation-schedule)
7. [Success Criteria](#success-criteria)

---

## Current State Assessment

### Component Status

| Component | Current State | Critical Issues |
|-----------|---------------|-----------------|
| **RustRipper** | Partially functional | Polling-based detection, unused dependencies, no integration |
| **makemkvstarter** | Broken | Incorrect MakeMKV CLI arguments |
| **ripper_starter** | Functional | Basic flatpak setup, limited config |
| **OMDb_API** | Partially functional | Incomplete data model, no URL encoding, single source |

### Current Architecture (Disconnected)

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│   RustRipper    │  ?  │   OMDb_API   │  ?  │ makemkvstarter  │
│ (Disc Detect)   │     │  (Metadata)  │     │   (Ripping)     │
└─────────────────┘     └──────────────┘     └─────────────────┘
        ?                                            ?
        └────────────────────┬───────────────────────┘
                             ▼
                    ┌──────────────┐
                    │  HandBrake   │
                    │ (Transcode)  │
                    └──────────────┘
```

---

## Target Architecture

### Containerized Web-Based Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         HOST SYSTEM (Fedora Linux)                       │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                    PODMAN POD: rustripper                          │ │
│  │                                                                    │ │
│  │  ┌──────────────────────┐  ┌───────────────────────────────────┐ │ │
│  │  │   Web UI (Svelte)    │  │   Backend API (Rust/Axum)         │ │ │
│  │  │  ┌────────────────┐  │  │  ┌────────────────────────────┐  │ │ │
│  │  │  │  Dashboard     │  │◀─┼─▶│  REST API + WebSocket      │  │ │ │
│  │  │  │  Job Queue     │  │  │  │  Disc Watcher (/dev/sr0)   │  │ │ │
│  │  │  │  Live Progress │  │  │  │  Job Orchestrator          │  │ │ │
│  │  │  │  History       │  │  │  │  Metadata APIs (TMDb/etc)  │  │ │ │
│  │  │  │  Config Editor │  │  │  │  SQLite Database           │  │ │ │
│  │  │  └────────────────┘  │  │  └────────────────────────────┘  │ │ │
│  │  │       :8080          │  │         :8081                    │ │ │
│  │  └──────────────────────┘  └───────────────────────────────────┘ │ │
│  │                                                                    │ │
│  │  ┌──────────────────────┐  ┌───────────────────────────────────┐ │ │
│  │  │  MakeMKV Worker      │  │   FFmpeg Worker                   │ │ │
│  │  │  - CLI execution     │  │   - Transcoding jobs              │ │ │
│  │  │  - Progress parsing  │  │   - Hardware accel (GPU)          │ │ │
│  │  │  - libdvdcss/libaacs │  │   - Priority queue                │ │ │
│  │  └──────────────────────┘  └───────────────────────────────────┘ │ │
│  │                                                                    │ │
│  │  ┌──────────────────────────────────────────────────────────────┐ │ │
│  │  │                    Shared Volumes                            │ │ │
│  │  │  📀 /dev/sr0 (device passthrough)                            │ │ │
│  │  │  💾 /mnt/rips (output directory)                             │ │ │
│  │  │  🗄️  /var/lib/rustripper (SQLite + config)                  │ │ │
│  │  │  🔑 ~/.config/aacs (KEYDB.cfg)                               │ │ │
│  │  └──────────────────────────────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  🌐 Access: http://localhost:8080 (or from any device on network)       │
└─────────────────────────────────────────────────────────────────────────┘

                           ┌─────────────────┐
                           │  WORKFLOW       │
                           └─────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
   DETECT (Backend)          IDENTIFY (Backend)          RIP (MakeMKV)
   - Poll /dev/sr0           - Parse disc label          - Container exec
   - Read label (blkid)      - Query TMDb/AniList        - Progress stream
   - Add to job queue        - Download metadata         - Output to volume
        │                           │                           │
        └───────────────────────────┼───────────────────────────┘
                                    ▼
                            TRANSCODE (FFmpeg)
                            - Container exec
                            - Hardware accel
                            - Progress stream
                                    ▼
                            FINALIZE (Backend)
                            - Generate thumbnails
                            - Update history DB
                            - WebSocket notification
```

---

## External Dependencies

> **These must be installed BEFORE MasterRustRipper can function properly**

### Decryption Libraries (System-Level)

#### libdvdcss (DVD Decryption)

| Distro | Installation Command |
|--------|---------------------|
| Fedora | See below (requires RPM Fusion) |
| Ubuntu/Debian | `sudo apt install libdvd-pkg && sudo dpkg-reconfigure libdvd-pkg` |
| Arch | `sudo pacman -S libdvdcss` |

**Fedora Installation:** Requires RPM Fusion repositories. See [code_schema.md](code_schema.md#fedora-libdvdcss-installation) for detailed commands.

**Notes:**
- MakeMKV uses libdvdcss automatically when available
- Fallback: MakeMKV has built-in decryption (may require key updates)
- Debug: Set `DVDCSS_VERBOSE=2` environment variable

#### libaacs & libbdplus (Blu-ray Decryption)

| Distro | Installation Command |
|--------|---------------------|
| Fedora | `sudo dnf install libaacs libbdplus` |
| Ubuntu/Debian | `sudo apt install libaacs0 libbdplus0` |
| Arch | `sudo pacman -S libaacs libbdplus` |

#### KEYDB.cfg (Blu-ray Key Database)

**Installation:** Download and install KEYDB.cfg to `~/.config/aacs/`. See [code_schema.md](code_schema.md#keydbcfg-download) for commands.

**Source:** https://forum.makemkv.com/forum/viewtopic.php?t=1307

---

### FFmpeg (Transcoding Engine)

| Distro | Installation Command |
|--------|---------------------|
| Fedora | `sudo dnf install ffmpeg` |
| Ubuntu/Debian | `sudo apt install ffmpeg` |
| Flatpak | `flatpak install flathub org.ffmpeg.FFmpeg` |

**Capabilities:**
- **Transcoding:** H.265 (HEVC), H.264 (AVC) encoding
- **Hardware Acceleration:** NVIDIA (NVENC), Intel (QuickSync), AMD (VCE/AMF)
- **Analysis:** ffprobe for media info extraction
- **Stream Management:** Extract/remove audio/subtitle tracks
- **Thumbnails:** Generate preview images

**Example Commands:** See [code_schema.md](code_schema.md#ffmpeg-commands) for encoding examples with software and hardware acceleration.

---

### Metadata APIs

| Source | Type | Auth Required | Best For |
|--------|------|---------------|----------|
| **TMDb** | REST | API Key (free) | Movies, TV shows, images |
| **OMDb** | REST | API Key (free) | Movies (basic info) |
| **AniList** | GraphQL | None | Anime |
| **Jikan (MAL)** | REST | None | Anime (alternative) |
| **TheTVDB** | REST | API Key (free) | TV series episodes |

**Priority Order:** TMDb → AniList → OMDb → TheTVDB

---

## Implementation Phases

> **⚠️ IMPORTANT:** Build core functionality as library crates FIRST, then containerize, then add Web UI.
> This allows testing each layer before adding complexity.

---

### Phase 1: Core Library Foundation (No Containers Yet)
> **Goal:** Create reusable Rust libraries that work standalone
> **Duration:** 2 weeks
> **Why First:** Validate business logic before containerization complexity

#### 1.1 Create Workspace Structure

**New Structure:**
```
RustRipper/
├── Cargo.toml (workspace)
├── crates/
│   ├── core/           # Core types and traits
│   ├── disc/           # Disc detection logic
│   ├── metadata/       # API clients (TMDb, AniList, etc.)
│   ├── ripper/         # MakeMKV wrapper
│   ├── transcode/      # FFmpeg wrapper
│   └── storage/        # Database and file operations
├── cli/                # Optional CLI binary (for testing)
├── backend/            # Web API server (Phase 3)
├── frontend/           # Web UI (Phase 4)
└── containers/         # Dockerfiles (Phase 2)
```

**Tasks:**
- [ ] Create workspace `Cargo.toml` with members
- [ ] Initialize library crates with basic structure
- [ ] Define common error types in `core` crate
- [ ] Set up workspace-level dependencies

#### 1.2 Core Types & Error Handling

**Files:** `crates/core/src/{lib.rs, error.rs, types.rs}`

**Key Components:**
- MediaType enum (Movie, TVShow, Anime, Music, Unknown)
- MediaInfo struct with title, year, poster URL, IMDb ID
- RipperError types using `thiserror` for all error scenarios
- Disc information structures
- Job status and progress types

**Tasks:**
- [ ] Define all error types using `thiserror`
- [ ] Create shared type definitions
- [ ] Implement serialization for API communication
- [ ] Create configuration structures

#### 1.3 Disc Detection Library

**File:** `crates/disc/src/lib.rs`

**Functionality:**
- DiscWatcher struct to monitor optical drive
- Poll device at configurable intervals (default 2 seconds)
- Execute `blkid` command to read disc information
- Parse disc label, type (DVD/Blu-ray), and filesystem info
- Return structured DiscInfo with device path, label, and disc type

**Tasks:**
- [ ] Move disc detection logic from `RustRipper/src/disc_detect.rs`
- [ ] Use `blkid` via `std::process::Command`
- [ ] Add proper error handling for missing devices
- [ ] Write unit tests with mock device responses

#### 1.4 MakeMKV Wrapper Library

**File:** `crates/ripper/src/lib.rs`

**Critical Fix:** Correct MakeMKV CLI arguments
- Current broken format: `--input=/dev/sr0 --output=/path`
- Correct format: `dev:/dev/sr0 <title> <output_dir> --minlength=<seconds>`

**Functionality:**
- MakeMKVRipper struct to execute ripping operations
- Parse stdout/stderr for progress percentages
- Stream progress updates to caller
- Handle MakeMKV-specific error codes
- Support title selection ("all" or specific title number)
- Configurable minimum title length filter

**Tasks:**
- [ ] Fix MakeMKV CLI argument format
- [ ] Add progress parsing from stdout
- [ ] Handle MakeMKV error codes
- [ ] Add title selection support
- [ ] Test with actual disc ripping

#### 1.5 Metadata API Library

**File:** `crates/metadata/src/lib.rs`

**Architecture:**
- MetadataProvider trait for consistent API across sources
- Individual modules: `tmdb.rs`, `anilist.rs`, `omdb.rs`, `tvdb.rs`
- MetadataAggregator to query multiple sources in priority order
- Disc label parser to sanitize and extract information

**Critical Fixes:**
- OMDb response struct needs proper serde rename attributes for PascalCase fields
- URL encoding for special characters in search queries
- Handle API error responses gracefully

**API Implementations:**
- **TMDb**: REST API for movies and TV shows
- **AniList**: GraphQL API for anime (no API key required)
- **OMDb**: REST API for basic movie info
- **TheTVDB**: REST API for TV series details

**Tasks:**
- [ ] Fix OMDb response struct with proper serde attributes
- [ ] Add URL encoding for search queries
- [ ] Implement TMDb API client with pagination
- [ ] Implement AniList GraphQL client
- [ ] Create aggregator that queries sources in priority order
- [ ] Add disc label sanitization (remove underscores, extract year)
- [ ] Implement media type detection (Movie/TV/Anime)

---

### Phase 2: CLI Testing Binary (Optional but Recommended)
> **Goal:** Test core libraries work correctly before containerization
> **Duration:** 1 week
> **Why Now:** Validate logic without API/container complexity

#### 2.1 Create CLI Binary

**File:** `cli/src/main.rs`

**Purpose:** Test and validate core libraries before adding web interface complexity

**Commands:**
- `watch`: Monitor optical drive and auto-rip on disc insertion
- `rip`: Manually rip current disc with optional title selection
- `search`: Test metadata API lookups by query
- `transcode`: Test FFmpeg transcoding with various presets

**Features:**
- Use `clap` for command-line argument parsing
- Display progress bars with `indicatif`
- Colorized output for better readability
- Verbose logging option for debugging
- Configuration file support

**Tasks:**
- [ ] Create CLI binary using `clap`
- [ ] Implement `watch` command (disc detection loop)
- [ ] Implement `rip` command (manual rip)
- [ ] Implement `search` command (test metadata APIs)
- [ ] Add progress bars with `indicatif`
- [ ] Add logging initialization

#### 2.2 Integration Testing

**Tasks:**
- [ ] Test disc detection with real optical drive
- [ ] Test MakeMKV execution with test disc
- [ ] Test metadata API calls
- [ ] Verify error handling
- [ ] Test with various disc labels
- [ ] Document any issues found

#### 2.3 FFmpeg Transcoding Library

**File:** `crates/transcode/src/lib.rs`

**Functionality:**
- FFmpegTranscoder struct to handle video transcoding
- Execute `ffprobe` to analyze media files (codec, resolution, duration, streams)
- Execute `ffmpeg` with configurable presets
- Parse stderr for progress updates (frame, fps, time, bitrate, speed)
- Detect available hardware encoders (NVENC, QuickSync, AMF)
- Calculate ETA based on current speed

**Preset System:**
- Balanced: H.265, CRF 20, medium (50-60% size reduction)
- HighQuality: H.265, CRF 18, slow (40-50% size reduction)
- Fast: H.265, CRF 23, fast (60-70% size reduction)
- Compatible: H.264, CRF 18 (maximum compatibility)
- HardwareAuto: Use detected GPU for encoding
- PassThrough: Copy streams without re-encoding

**Tasks:**
- [ ] Implement FFmpeg wrapper with progress parsing
- [ ] Add hardware acceleration detection
- [ ] Create preset system (balanced, quality, fast)
- [ ] Implement ffprobe media analysis
- [ ] Test transcoding with sample files
- [ ] Add thumbnail generation support

---

### Phase 3: Backend API Server (Axum)
> **Goal:** Create REST API and WebSocket server using core libraries
> **Duration:** 2 weeks
> **Why Now:** API layer before containers allows local testing

#### 3.1 Project Structure

**Directory:** `backend/`

**Structure:** See [code_schema.md](code_schema.md#backend-project-structure) for complete directory layout.

**Key Modules:**
- `api/` - REST endpoints, WebSocket handler, middleware
- `jobs/` - Job queue management and execution logic
- `disc/` - Background disc monitoring
- `db/` - Database schema and queries

#### 3.2 REST API Endpoints

**File:** `backend/src/api/routes.rs`

**Endpoint Groups:**

**System Status:**
- `GET /api/status` - Current system state (disc present, active jobs, system resources)
- `GET /api/system/health` - Health check for monitoring

**Job Management:**
- `GET /api/jobs` - List all jobs with filtering options
- `POST /api/jobs` - Create new rip job
- `GET /api/jobs/:id` - Get specific job details
- `DELETE /api/jobs/:id` - Cancel running job
- `PUT /api/jobs/:id/priority` - Change job priority

**History:**
- `GET /api/history` - Browse rip history with pagination
- `GET /api/history/:id` - Get specific rip details
- `GET /api/history/stats` - Statistics (total rips, storage saved)

**Configuration:**
- `GET /api/config` - Get current configuration
- `PUT /api/config` - Update configuration (live reload)

**Metadata:**
- `POST /api/metadata/search` - Search for title across all sources

**WebSocket:**
- `WS /ws` - Real-time updates (disc events, job progress)

**Tasks:**
- [ ] Set up Axum server with router
- [ ] Implement all REST endpoints
- [ ] Add CORS middleware for web UI
- [ ] Add request logging with tracing
- [ ] Implement consistent error responses
- [ ] Add OpenAPI/Swagger documentation

#### 3.3 WebSocket for Real-Time Updates

**File:** `backend/src/api/websocket.rs`

**Message Types:**
- DiscInserted: New disc detected with label
- DiscEjected: Disc removed from drive
- JobStarted: Rip job began with title info
- JobProgress: Progress update (percentage, stage, ETA)
- JobCompleted: Job finished successfully with output path
- JobFailed: Job failed with error message
- SystemStatus: Periodic system metrics (CPU, memory, disk space)

**Architecture:**
- Use tokio broadcast channel for pub/sub pattern
- Multiple WebSocket clients can connect simultaneously
- Backend emits events to broadcast channel
- WebSocket handler subscribes and forwards to client
- JSON-serialized messages with type tag

**Tasks:**
- [ ] Implement WebSocket upgrade handler
- [ ] Set up broadcast channel for events
- [ ] Emit events from disc watcher
- [ ] Emit events from job executor
- [ ] Handle client disconnections gracefully
- [ ] Add heartbeat/ping-pong to detect dead connections
- [ ] Add reconnection support on client side

#### 3.4 Job Queue System

**File:** `backend/src/jobs/queue.rs`

**Architecture:**
- In-memory VecDeque for active job queue
- SQLite for job persistence across restarts
- Job states: Queued → Ripping → Transcoding → Completed/Failed
- Automatic state transitions with event emission
- Support for job priority reordering
- Job cancellation with cleanup

**Job Structure:**
- Unique ID (auto-increment)
- Disc label (raw from device)
- Resolved title and year (from metadata)
- Status and progress (0.0-100.0)
- Stage (detecting, ripping, transcoding, finalizing)
- Timestamps (created, started, completed)
- Error message if failed

**Executor Pattern:**
- Background tokio task processes queue
- Dequeue next job when worker available
- Execute rip → transcode → finalize pipeline
- Update job status in database
- Emit WebSocket events for UI updates
- Handle errors and retry logic

**Tasks:**
- [ ] Implement in-memory job queue with VecDeque
- [ ] Persist jobs to SQLite on state changes
- [ ] Handle job state transitions
- [ ] Emit WebSocket events on status changes
- [ ] Implement job executor background task
- [ ] Add job cancellation support
- [ ] Prevent duplicate jobs for same disc

#### 3.5 Background Disc Watcher

**File:** `backend/src/disc/watcher.rs`

**Functionality:**
- Long-running background tokio task
- Poll disc device every 2 seconds using disc detection library
- Track last known disc state to detect insertions/ejections
- Emit WebSocket events on state changes
- Auto-create rip jobs on disc insertion (configurable)

**Workflow on Disc Insertion:**
1. Detect new disc via label change
2. Log disc insertion event
3. Fetch metadata from aggregator
4. Check history database for duplicates
5. Create new job if not duplicate (or user preference)
6. Emit DiscInserted WebSocket event
7. Enqueue job to JobQueue

**Duplicate Detection:**
- Generate unique disc ID (hash of label + disc type)
- Check against history database
- Allow user to override and re-rip if desired
- Configurable auto-skip behavior

**Tasks:**
- [ ] Spawn background tokio task for disc polling
- [ ] Track last disc state to detect changes
- [ ] Auto-create jobs when disc inserted
- [ ] Emit WebSocket events for disc insertion/ejection
- [ ] Implement duplicate detection
- [ ] Handle graceful shutdown with tokio cancellation token
- [ ] Add configurable auto-rip behavior

#### 3.6 SQLite Database

**File:** `backend/src/db/schema.sql`

**Tables:**

**jobs**: Active and historical job records
- Job ID, disc label, resolved title/year
- Status (queued, ripping, transcoding, completed, failed)
- Progress percentage and current stage
- Timestamps (created, started, completed)
- Error message if failed

**rips**: Completed rip statistics
- Links to job via foreign key
- Unique disc ID for duplicate detection
- Output file path
- Original and final file sizes
- Duration in seconds
- Completion timestamp

**config**: Dynamic configuration storage
- Key-value pairs with JSON values
- Updated timestamp for tracking changes
- Allows live configuration updates without restart

**Indexes:**
- jobs.status for filtering active jobs
- rips.disc_id for duplicate detection
- jobs.created_at for chronological sorting

**Tasks:**
- [ ] Set up `sqlx` with compile-time query checking
- [ ] Create database schema and migrations
- [ ] Create database helper functions (insert, update, query)
- [ ] Implement config storage/retrieval with JSON values
- [ ] Add job persistence for crash recovery
- [ ] Add history tracking with statistics
- [ ] Add database cleanup for old completed jobs

---

### Phase 4: Containerization with Podman
> **Goal:** Package backend and workers into containers
> **Duration:** 1-2 weeks
> **Why Now:** Core functionality tested, ready to isolate dependencies

#### 4.1 Backend Dockerfile

**File:** `backend/Dockerfile`

**Multi-Stage Build Strategy:**
- **Stage 1 (builder)**: Use rust:1.75-slim to compile workspace
- **Stage 2 (runtime)**: Use debian:bookworm-slim for minimal image
- Copy only compiled binary and required runtime libraries

**Optimizations:**
- Layer caching: Copy Cargo.toml files before source code
- Incremental compilation for faster rebuilds during development
- Strip debug symbols in release build
- Install only essential runtime dependencies (libsqlite3, ca-certificates)

**Configuration:**
- Expose port 8081 for API server
- Use environment variables for runtime config
- Mount volumes for database and output directory

**Tasks:**
- [ ] Create multi-stage Dockerfile for backend
- [ ] Optimize layer caching (copy Cargo.toml first)
- [ ] Minimize final image size (<100MB target)
- [ ] Test local build: `podman build -t rustripper-backend ./backend`
- [ ] Test container run with volume mounts
- [ ] Document build and run commands

#### 4.2 MakeMKV Worker Dockerfile

**File:** `containers/makemkv/Dockerfile`

**Base Image:** Ubuntu 22.04 (provides necessary build tools and libraries)

**Installation Strategy:**
- **Option 1**: Build MakeMKV from source (latest version, more control)
- **Option 2**: Use pre-built packages from PPA (faster builds)

**Required Components:**
- MakeMKV binaries (makemkvcon CLI tool)
- Decryption libraries: libdvdcss2, libaacs0, libbdplus0
- Build dependencies if compiling from source
- QT libraries (required even for CLI)

**Device Access:**
- Container needs `/dev/sr0` device passthrough
- Must run in privileged mode or with specific capabilities
- Access to KEYDB.cfg via mounted volume

**Tasks:**
- [ ] Create MakeMKV container with all dependencies
- [ ] Include libdvdcss, libaacs, libbdplus
- [ ] Test device passthrough with real optical drive
- [ ] Verify ripping works inside container
- [ ] Test with encrypted DVD and Blu-ray
- [ ] Optimize image size by removing build dependencies

#### 4.3 FFmpeg Worker Container

**Recommended Approach:** Use existing `linuxserver/ffmpeg:latest` image

**Why Pre-Built Image:**
- Professionally maintained and regularly updated
- Includes all codecs and filters
- Optimized build flags for performance
- Hardware acceleration support included
- Smaller than custom-built alternatives

**Alternative: Custom Build for Specific GPU Support**
- For NVIDIA: Use nvidia/cuda base image
- For Intel: Include Intel Media SDK
- For AMD: Include AMF/VCE support
- Only needed if linuxserver image doesn't work

**GPU Passthrough:**
- NVIDIA: Requires nvidia-container-toolkit and /dev/nvidia* devices
- Intel QuickSync: Mount /dev/dri device
- AMD: Mount /dev/dri and /dev/kfd devices

**Tasks:**
- [ ] Choose FFmpeg image (linuxserver/ffmpeg recommended)
- [ ] Test GPU passthrough for hardware encoding
- [ ] Verify NVENC/QuickSync/AMF availability in container
- [ ] Test actual transcoding with sample file
- [ ] Benchmark hardware vs software encoding performance
- [ ] Document GPU setup requirements for users

#### 4.4 Podman Compose Configuration

**File:** `podman-compose.yml`

**Services Architecture:**

**backend**: Rust API server
- Build from local Dockerfile
- Device passthrough for /dev/sr0
- Mount volumes: rips (output), db (SQLite), aacs keys
- Environment: logging level, paths, database URL
- Expose port 8081 for API
- Restart policy: unless-stopped

**makemkv**: MakeMKV worker
- Build from containers/makemkv/Dockerfile
- Device passthrough for /dev/sr0
- Mount volumes: rips (output), aacs keys (read-only)
- Privileged mode required for device access
- No exposed ports (command execution via podman exec)

**ffmpeg**: Transcoding worker
- Use linuxserver/ffmpeg:latest image
- Mount volume: rips (read/write)
- Device passthrough for GPU (/dev/dri)
- Environment: PUID/PGID for file permissions
- No exposed ports (command execution via podman exec)

**Volume Configuration:**
- **rips**: Bind mount to host directory (e.g., ~/Videos/Ripped)
- **db**: Named volume for SQLite database persistence
- **aacs**: Bind mount to ~/.config/aacs (read-only) for KEYDB.cfg

**Network:**
- Bridge network for inter-container communication
- Containers can reach each other by service name
- Only backend port exposed to host

**Tasks:**
- [ ] Create podman-compose.yml with all services
- [ ] Configure volume mounts and bind mounts
- [ ] Set up device passthrough for optical drive and GPU
- [ ] Configure networking between containers
- [ ] Set appropriate file permissions (PUID/PGID)
- [ ] Test: `podman-compose up -d`
- [ ] Verify all containers start successfully
- [ ] Test inter-container communication
- [ ] Document environment variables and customization

#### 4.5 Container Communication Strategy

**Approach: Backend Orchestrates Worker Containers**

**Method 1: Direct Execution (Recommended)**
- Backend executes `podman exec` commands to run jobs in worker containers
- Capture stdout/stderr for progress parsing
- Stream output back to job queue for UI updates
- Simpler architecture, no additional infrastructure

**Method 2: Shared Queue Files**
- Workers poll shared volume for job files
- Backend writes job descriptions as JSON
- Workers execute and update status files
- More decoupled but requires polling overhead

**Implementation Details:**

**MakeMKV Execution:**
- Backend calls: `podman exec rustripper-makemkv makemkvcon mkv dev:/dev/sr0 all /mnt/rips`
- Parse stdout for progress percentages
- Handle exit codes for errors
- Stream progress to WebSocket clients

**FFmpeg Execution:**
- Backend calls: `podman exec rustripper-ffmpeg ffmpeg -i input.mkv [args] output.mkv`
- Parse stderr for frame/time/speed info
- Calculate ETA from speed metric
- Update job progress in database

**Error Handling:**
- Capture stderr for error messages
- Parse MakeMKV/FFmpeg error codes
- Retry logic for transient failures
- Clean up partial files on failure

**Tasks:**
- [ ] Implement `podman exec` commands from backend
- [ ] Parse stdout/stderr from container exec in real-time
- [ ] Handle container errors and exit codes
- [ ] Stream progress updates to WebSocket
- [ ] Add retry logic for common errors
- [ ] Test error scenarios (disc removed, out of space)
- [ ] Alternative: Implement shared queue file system if needed

---

### Phase 5: Web UI Frontend
> **Goal:** Build responsive web interface for monitoring and control
> **Duration:** 2-3 weeks
> **Why Last:** Backend/containers must be stable before UI development

#### 5.1 Frontend Technology Selection and Setup

**Recommended: SvelteKit**
- Smallest bundle size (better performance on mobile)
- Excellent reactivity for real-time updates
- Server-side rendering support
- TypeScript integration
- Great developer experience

**Alternative: React + Vite**
- Larger ecosystem and community
- More third-party component libraries
- Familiar to more developers

**Alternative: Vue 3 + Vite**
- Good balance of features and simplicity
- Composition API is clean and intuitive

**Setup Steps:**
- Initialize project with chosen framework
- Install Tailwind CSS for styling
- Set up TypeScript configuration
- Configure build tools (Vite/webpack)
- Set up ESLint and Prettier

**Project Structure:**
- `/routes` - Page components (dashboard, jobs, history, settings)
- `/lib/components` - Reusable UI components
- `/lib/api.ts` - REST API client wrapper
- `/lib/websocket.ts` - WebSocket connection manager
- `/lib/stores` - State management (Svelte stores or Zustand/Redux)

**Tasks:**
- [ ] Choose and initialize frontend framework
- [ ] Set up Tailwind CSS and design system
- [ ] Configure TypeScript
- [ ] Create project structure
- [ ] Set up development server with hot reload
- [ ] Configure proxy for API calls during development

#### 5.2 API Client Layer

**REST API Client:**
- Wrapper functions for all backend endpoints
- Automatic error handling and retries
- Request/response type definitions
- Loading state management
- Error state handling

**WebSocket Client:**
- Auto-connect on page load
- Reconnection logic with exponential backoff
- Event type parsing and dispatching
- Connection state tracking (connected, disconnected, reconnecting)
- Heartbeat/ping-pong for connection health

**State Management:**
- Global application state (jobs, disc status, system info)
- Real-time updates from WebSocket
- Optimistic UI updates
- Cache invalidation strategies

**Tasks:**
- [ ] Create API client with fetch wrapper
- [ ] Implement WebSocket connection manager
- [ ] Set up state management solution
- [ ] Add type definitions for all API responses
- [ ] Implement automatic reconnection logic
- [ ] Add request caching where appropriate

#### 5.3 Core UI Components

**Reusable Components:**
- **ProgressBar**: Animated progress with percentage display
- **StatusBadge**: Color-coded status indicators (queued, ripping, etc.)
- **JobCard**: Display job information with actions
- **SystemStatus**: CPU, memory, disk space indicators
- **DiscIndicator**: Show disc present/absent state
- **MediaCard**: Display media info with poster image
- **ConfirmDialog**: Confirmation for destructive actions
- **NotificationToast**: Success/error notifications

**Styling Approach:**
- Use Tailwind utility classes for consistency
- Create custom CSS for complex animations
- Dark mode support (optional but nice)
- Responsive design for mobile/tablet/desktop
- Accessible (ARIA labels, keyboard navigation)

**Tasks:**
- [ ] Create all reusable UI components
- [ ] Implement loading states and skeletons
- [ ] Add hover effects and transitions
- [ ] Test components in isolation
- [ ] Ensure responsive behavior on all screen sizes
- [ ] Add accessibility features

#### 5.4 Dashboard Page

**Primary View - Real-Time Status:**

**Header Section:**
- Application title and logo
- Navigation menu
- Settings button
- System health indicator

**Current Activity Panel:**
- Show active rip job if running
- Live progress bar with percentage
- Current stage (detecting, ripping, transcoding)
- Speed metric and ETA
- Cancel button

**Disc Status Panel:**
- Visual indicator for disc present/absent
- Disc label when present
- Detected media type (Movie/TV/Anime)
- Metadata preview if fetched

**Job Queue Section:**
- List of queued jobs
- Drag-to-reorder functionality
- Pause/resume/cancel actions
- Priority indicators
- Empty state when no jobs

**System Status Footer:**
- CPU usage graph
- Memory usage graph
- Disk space available
- Container health status

**Tasks:**
- [ ] Design dashboard layout (wireframe)
- [ ] Implement dashboard components
- [ ] Connect to WebSocket for real-time updates
- [ ] Add auto-refresh for static data
- [ ] Test with various job states
- [ ] Optimize rendering performance

#### 5.5 Job Queue Management Page

**Features:**
- Table view of all jobs (queued, active, completed)
- Filter by status (all, active, completed, failed)
- Sort by date, priority, status
- Search by title
- Bulk actions (cancel multiple, clear completed)
- Pagination for large job lists

**Job Details Modal:**
- Full job information
- Progress timeline
- Logs/errors if failed
- Metadata details
- Output file path
- File size statistics

**Tasks:**
- [ ] Create job table with sorting/filtering
- [ ] Implement search functionality
- [ ] Add bulk action checkboxes
- [ ] Create job details modal
- [ ] Add pagination controls
- [ ] Test with large job counts

#### 5.6 History Browser Page

**Features:**
- Browse all completed rips
- Date range filter
- Media type filter (Movies/TV/Anime)
- Search by title
- Grid or list view toggle
- Statistics dashboard (total rips, storage saved)

**Rip Details View:**
- Media poster image
- Title, year, genre info
- Original vs final file size
- Compression ratio
- Rip date
- Output file path
- Re-rip button if needed

**Statistics Panel:**
- Total discs ripped
- Total storage saved (GB)
- Average compression ratio
- Most common media types
- Chart of rips over time

**Tasks:**
- [ ] Create history list/grid view
- [ ] Implement filtering and search
- [ ] Add date range picker
- [ ] Create statistics dashboard with charts
- [ ] Implement infinite scroll or pagination
- [ ] Add export functionality (CSV/JSON)

#### 5.7 Settings/Configuration Page

**Configuration Sections:**

**General Settings:**
- Output directory path
- Disc device path (/dev/sr0)
- Auto-eject after rip
- Check for duplicates
- Auto-start rip on disc insertion

**MakeMKV Settings:**
- Executable path
- Minimum title length (seconds)
- Title selection (all or specific)

**FFmpeg Settings:**
- Enable transcoding toggle
- Codec selection (H.264, H.265, hardware)
- Preset selection (fast, balanced, quality)
- CRF value slider
- Hardware acceleration selection
- Keep original files toggle

**Metadata Settings:**
- API key inputs (TMDb, OMDb, TheTVDB)
- Source priority order (drag-to-reorder)
- Preferred language
- Download artwork toggle

**Notification Settings:**
- Desktop notifications (if supported)
- Sound alerts toggle

**Form Behavior:**
- Live validation
- Save button (commits to backend)
- Reset to defaults button
- Test buttons for API keys

**Tasks:**
- [ ] Create form sections for all settings
- [ ] Implement form validation
- [ ] Connect to backend config API
- [ ] Add save/cancel/reset functionality
- [ ] Add API key test buttons
- [ ] Show success/error notifications
- [ ] Add help text and tooltips

#### 5.8 Frontend Dockerfile and Deployment

**Multi-Stage Build:**
- **Stage 1**: Build frontend with npm/node
- **Stage 2**: Serve static files with nginx

**Nginx Configuration:**
- Serve built static files
- Proxy /api/* requests to backend:8081
- Proxy /ws to backend:8081 for WebSocket
- Enable gzip compression
- Set appropriate cache headers

**Build Optimizations:**
- Tree-shaking to remove unused code
- Code splitting for faster initial load
- Asset optimization (image compression, etc.)
- Minification and bundling

**Tasks:**
- [ ] Create multi-stage Dockerfile
- [ ] Configure nginx for SPA routing
- [ ] Set up API proxy in nginx
- [ ] Optimize build for production
- [ ] Test built container locally
- [ ] Add frontend to podman-compose.yml
- [ ] Test full stack with all containers

---

### Phase 6: FFmpeg Integration
> **Goal:** Reduce file sizes through transcoding

#### 6.1 FFmpeg Wrapper Module

**File:** `shared_lib/src/ffmpeg.rs`

```rust
pub struct MediaInfo {
    pub duration: Duration,
    pub video_codec: String,
    pub video_resolution: (u32, u32),
    pub audio_tracks: Vec<AudioTrack>,
    pub subtitle_tracks: Vec<SubtitleTrack>,
    pub file_size: u64,
}

pub struct TranscodeSettings {
    pub codec: VideoCodec,
    pub preset: EncoderPreset,
    pub crf: u8,
    pub hardware_accel: HardwareAccel,
    pub audio_codec: AudioCodec,
    pub audio_bitrate: String,
}

pub fn probe_media(path: &Path) -> Result<MediaInfo>
pub fn transcode(input: &Path, output: &Path, settings: &TranscodeSettings, progress_callback: F) -> Result<()>
pub fn detect_hardware_accel() -> Vec<HardwareAccel>
pub fn generate_thumbnail(input: &Path, output: &Path, timestamp: Duration) -> Result<()>
pub fn extract_streams(input: &Path, output: &Path, streams: &[StreamSelector]) -> Result<()>
```

#### 6.2 Video Codecs & Hardware Acceleration

```rust
pub enum VideoCodec {
    // Software encoders
    H264,           // libx264
    H265,           // libx265
    
    // NVIDIA NVENC
    H264Nvenc,
    H265Nvenc,
    
    // Intel QuickSync
    H264Qsv,
    H265Qsv,
    
    // AMD AMF/VCE
    H264Amf,
    H265Amf,
}

pub enum HardwareAccel {
    None,
    Nvidia,
    Intel,
    Amd,
    Auto,
}
```

#### 6.3 Transcoding Presets

```rust
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
    
    /// User-defined settings from config
    Custom,
}
```

#### 6.4 Progress Parsing

Parse FFmpeg stderr for progress:
```
frame=12345 fps=120 q=22.0 size=1234567kB time=00:30:00.00 bitrate=5000kbps speed=2.5x
```

**Tasks:**
- [ ] Detect available hardware encoders
- [ ] Implement preset system
- [ ] Parse progress output in real-time
- [ ] Calculate ETA based on speed
- [ ] Report size reduction percentage

---

### Phase 7: Orchestration & Workflow Integration
> **Goal:** Connect all components into unified workflow

#### 7.1 Orchestrator Module

**File:** `RustRipper/src/orchestrator.rs`

```rust
pub enum WorkflowState {
    Idle,
    DiscDetected { device: String, label: String },
    FetchingMetadata { label: String },
    PreparingOutput { title: String, year: Option<u16> },
    Ripping { title: String, progress: f32 },
    Transcoding { title: String, progress: f32 },
    Finalizing { title: String },
    Complete { output_path: PathBuf, stats: RipStats },
    Error { stage: String, message: String },
}

pub struct RipStats {
    pub original_size: u64,
    pub final_size: u64,
    pub duration: Duration,
    pub media_info: MediaInfo,
}

pub struct Orchestrator {
    config: Config,
    state: WorkflowState,
    history: HistoryDb,
}

impl Orchestrator {
    pub async fn run(&mut self) -> Result<()>
    pub fn handle_disc_insert(&mut self, device: &str, label: &str)
    pub fn handle_disc_eject(&mut self)
    pub fn get_state(&self) -> &WorkflowState
}
```

#### 7.2 Complete Workflow

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. DETECT                                                        │
│    └─▶ Poll /dev/sr0 OR listen for udev events                  │
│    └─▶ Read disc label via blkid                                │
│    └─▶ Check if disc already in history (skip if duplicate)     │
├─────────────────────────────────────────────────────────────────┤
│ 2. IDENTIFY                                                      │
│    └─▶ Sanitize disc label (remove underscores, extract year)   │
│    └─▶ Detect media type (Movie/TV/Anime)                       │
│    └─▶ Query appropriate APIs based on type                     │
│    └─▶ Get metadata: title, year, artwork URL                   │
├─────────────────────────────────────────────────────────────────┤
│ 3. PREPARE                                                       │
│    └─▶ Create output directory: "{Title} ({Year})/"             │
│    └─▶ Download artwork (poster.jpg)                            │
│    └─▶ Generate .nfo file (optional, for media servers)         │
├─────────────────────────────────────────────────────────────────┤
│ 4. RIP                                                           │
│    └─▶ Execute MakeMKV with correct arguments                   │
│    └─▶ Parse progress output                                    │
│    └─▶ Handle errors (missing keys, protected disc)             │
├─────────────────────────────────────────────────────────────────┤
│ 5. TRANSCODE (if enabled)                                        │
│    └─▶ Analyze ripped file with ffprobe                         │
│    └─▶ Select best encoder (hardware if available)              │
│    └─▶ Apply FFmpeg transcoding with progress                   │
│    └─▶ Verify output integrity                                  │
│    └─▶ Delete original (if configured)                          │
├─────────────────────────────────────────────────────────────────┤
│ 6. FINALIZE                                                      │
│    └─▶ Generate thumbnail                                       │
│    └─▶ Record in history database                               │
│    └─▶ Send desktop notification                                │
│    └─▶ Eject disc (if configured)                               │
└─────────────────────────────────────────────────────────────────┘
```

#### 7.3 Output Directory Management

**Naming Templates:**
```toml
[output]
movie_template = "{title} ({year})"
tv_template = "{title}/Season {season}"
anime_template = "{title} ({year})"
```

**Example Output Structure:**
```
/home/user/Videos/Ripped/
├── Inception (2010)/
│   ├── Inception (2010).mkv
│   ├── poster.jpg
│   └── thumbnail.jpg
├── Cowboy Bebop (1998)/
│   ├── Cowboy Bebop (1998) - Disc 1.mkv
│   ├── poster.jpg
│   └── thumbnail.jpg
└── Breaking Bad/
    └── Season 1/
        ├── Breaking Bad S01E01.mkv
        └── ...
```

**Tasks:**
- [ ] Create directory with sanitized name
- [ ] Handle duplicate detection
- [ ] Support custom templates
- [ ] Sanitize filenames for filesystem

---

### Phase 8: Error Handling & Logging
> **Goal:** Robust error handling and observability

#### 8.1 Custom Error Types

**File:** `shared_lib/src/error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum RipperError {
    #[error("Disc not found at {0}")]
    DiscNotFound(String),
    
    #[error("Decryption failed: {0}. Is {1} installed?")]
    DecryptionFailed(String, String),
    
    #[error("KEYDB.cfg not found or outdated. Run 'ripper_starter' to update.")]
    KeyDbMissing,
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("MakeMKV error: {0}")]
    MakeMKVError(String),
    
    #[error("FFmpeg error: {0}")]
    FFmpegError(String),
    
    #[error("Metadata lookup failed for '{0}': {1}")]
    MetadataError(String, String),
    
    #[error("Hardware encoder {0} not available")]
    HardwareEncoderUnavailable(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}
```

#### 8.2 Structured Logging

```rust
// Initialize in main()
env_logger::Builder::from_env(
    env_logger::Env::default().default_filter_or("info")
).init();

// Usage throughout codebase
log::info!("Disc detected: {} ({})", device, label);
log::debug!("MakeMKV command: {:?}", command);
log::warn!("KEYDB.cfg is {} days old, consider updating", age);
log::error!("Ripping failed: {}", err);
```

**Log Levels:**
- `error` - Failures that stop the workflow
- `warn` - Issues that don't stop workflow but need attention
- `info` - Normal operation messages (disc detected, rip complete)
- `debug` - Detailed info for troubleshooting
- `trace` - Very verbose (command outputs, API responses)

#### 8.3 Graceful Shutdown

**Tasks:**
- [ ] Handle SIGINT (Ctrl+C) and SIGTERM
- [ ] Save current workflow state
- [ ] Clean up temporary files
- [ ] Allow resume on restart (for long transcodes)

---

### Phase 9: User Experience
> **Goal:** Progress indication, notifications, and history

#### 9.1 Progress Reporting

**Dependencies:** `indicatif = "0.17"`

```rust
let pb = ProgressBar::new(100);
pb.set_style(ProgressStyle::default_bar()
    .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}% | {msg}")
    .progress_chars("█▓░"));

// Ripping progress
pb.set_message("Ripping: Inception (2010)");
pb.set_position(45);

// Transcoding progress with ETA
pb.set_message("Transcoding: 2.5x speed, ETA 15:32");
```

#### 9.2 Desktop Notifications

**Dependencies:** `notify-rust = "4"`

```rust
use notify_rust::Notification;

// On rip start
Notification::new()
    .summary("Ripping Started")
    .body("Inception (2010)")
    .icon("media-optical")
    .show()?;

// On complete
Notification::new()
    .summary("Rip Complete")
    .body("Inception (2010)\nSize: 4.2 GB → 1.8 GB (57% reduction)")
    .icon("emblem-ok")
    .show()?;

// On error
Notification::new()
    .summary("Ripping Failed")
    .body("Could not decrypt disc. Check KEYDB.cfg")
    .icon("dialog-error")
    .urgency(notify_rust::Urgency::Critical)
    .show()?;
```

#### 9.3 History Database

**Dependencies:** `rusqlite = "0.31"`

**Location:** `~/.local/share/masterrustripper/history.db`

```sql
CREATE TABLE rips (
    id INTEGER PRIMARY KEY,
    disc_id TEXT UNIQUE,         -- Unique disc identifier
    title TEXT NOT NULL,
    year INTEGER,
    media_type TEXT,             -- movie, tv, anime
    rip_date DATETIME DEFAULT CURRENT_TIMESTAMP,
    output_path TEXT,
    original_size INTEGER,       -- bytes
    final_size INTEGER,          -- bytes after transcode
    status TEXT,                 -- completed, failed, in_progress
    error_message TEXT
);

CREATE INDEX idx_disc_id ON rips(disc_id);
CREATE INDEX idx_title ON rips(title);
```

**Tasks:**
- [ ] Create/migrate database schema
- [ ] Check disc_id before ripping (prevent duplicates)
- [ ] Record all rips with statistics
- [ ] Provide CLI to view history
- [ ] Calculate total storage saved

---

### Phase 10: HandBrake Integration (Optional)
> **Goal:** Alternative transcoding for users who prefer HandBrake presets

**File:** `shared_lib/src/handbrake.rs`

```rust
pub fn list_presets() -> Result<Vec<String>>
pub fn transcode(input: &Path, output: &Path, preset: &str) -> Result<()>
pub fn parse_progress(line: &str) -> Option<f32>
```

**Note:** FFmpeg is the primary transcoding tool. HandBrake is optional for users who prefer its preset system.

---

### Phase 11: Testing & Documentation
> **Goal:** Ensure reliability and usability

#### 11.1 Unit Tests

- [ ] Config parsing and validation
- [ ] Disc label sanitization (various formats)
- [ ] API response parsing (success and error cases)
- [ ] Command argument building
- [ ] Error type conversions

#### 11.2 Integration Tests

- [ ] Full workflow with mock disc
- [ ] Error recovery scenarios
- [ ] Config migration from old format
- [ ] Database operations

#### 11.3 Documentation

- [ ] README.md with quick start guide
- [ ] INSTALL.md with detailed setup
- [ ] CONFIG.md with all options explained
- [ ] TROUBLESHOOTING.md for common issues
- [ ] Architecture documentation

---

## Configuration Schema

### Complete Config File

**Location:** `~/.config/masterrustripper/config.toml`

**See:** [code_schema.md](code_schema.md#configuration-file-schema) for complete TOML configuration example.

**Key Sections:**
- `[general]` - Output path, disc device, auto-eject, duplicate checking
- `[makemkv]` - Executable path, title length, selection options
- `[ffmpeg]` - Codec, preset, CRF, hardware acceleration settings
- `[metadata]` - API keys and source priority for TMDb, AniList, OMDb, TheTVDB
- `[output]` - Naming templates for movies, TV shows, and anime
- `[notifications]` - Desktop and sound notification settings
- `[history]` - Database path and history tracking

---

## Implementation Schedule

### Week 1: Foundation
| Day | Tasks |
|-----|-------|
| 1-2 | Fix MakeMKV CLI arguments, test with real disc |
| 3-4 | Fix OMDb response struct, add URL encoding |
| 4-5 | Clean up RustRipper dependencies |

### Week 2: Infrastructure
| Day | Tasks |
|-----|-------|
| 1-2 | Create workspace Cargo.toml, shared_lib crate |
| 3-4 | Create error types and logging setup |
| 4-5 | Basic project structure, builds successfully |

### Week 3: System Dependencies
| Day | Tasks |
|-----|-------|
| 1-2 | Create system dependency checker module |
| 3-4 | Implement KEYDB.cfg management |
| 4-5 | Integrate dependency checks into ripper_starter |

### Week 4: Configuration
| Day | Tasks |
|-----|-------|
| 1-2 | Implement unified configuration system |
| 3-4 | Migrate all components to use shared config |
| 4-5 | Test config loading, defaults, validation |

### Week 5: Metadata System
| Day | Tasks |
|-----|-------|
| 1-2 | Create metadata aggregator architecture |
| 3 | Implement TMDb API integration |
| 4 | Implement AniList GraphQL integration |
| 5 | Implement disc label parser, media type detection |

### Week 6: FFmpeg Integration
| Day | Tasks |
|-----|-------|
| 1-2 | Create FFmpeg wrapper, implement ffprobe |
| 3-4 | Implement transcoding with presets |
| 4-5 | Add hardware acceleration detection, progress parsing |

### Week 7: Orchestration
| Day | Tasks |
|-----|-------|
| 1-2 | Create orchestrator module with state machine |
| 3-4 | Implement complete workflow end-to-end |
| 4-5 | Test with real disc, fix integration issues |

### Week 8: Polish & UX
| Day | Tasks |
|-----|-------|
| 1-2 | Add progress bars (indicatif) |
| 3-4 | Add desktop notifications |
| 4-5 | Implement history database |

### Week 9: Testing & Docs
| Day | Tasks |
|-----|-------|
| 1-2 | Write unit tests for all modules |
| 3-4 | Write integration tests |
| 4-5 | Documentation, README, troubleshooting guide |

### Week 10: Finalization
| Day | Tasks |
|-----|-------|
| 1-2 | Bug fixes from testing |
| 3-4 | Performance optimization |
| 4-5 | Release preparation, version tagging |

---

## Success Criteria

| Criteria | Metric |
|----------|--------|
| **Functional** | Insert disc → organized output folder with correct metadata |
| **Reliable** | Graceful error handling, no crashes, clear error messages |
| **Configurable** | Single config file controls all behavior |
| **Observable** | Progress bars, logging, desktop notifications |
| **Efficient** | FFmpeg transcoding achieves 50-70% file size reduction |
| **Compatible** | Successfully rips encrypted DVDs (libdvdcss) and Blu-rays (libaacs) |
| **Comprehensive** | Correctly identifies and tags Movies, TV Shows, and Anime |
| **Maintainable** | Clean code structure, proper error types, documented |

---

## Quick Start Commands

**See:** [code_schema.md](code_schema.md#build-and-deployment) for build, test, and container commands.

---

## Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Disc detection | Polling (2s interval) | Simpler, works reliably, potential for cross-platform |
| Primary transcoder | FFmpeg | More flexible, better hardware acceleration support |
| Primary metadata (movies) | TMDb | Better data quality, good free tier, images included |
| Primary metadata (anime) | AniList | No API key needed, comprehensive, GraphQL is modern |
| Configuration location | XDG paths | Standard Linux convention (`~/.config/`) |
| History storage | SQLite | Lightweight, embedded, no server needed |
| Error handling | thiserror | Clean derive macros, good ecosystem integration |
| Async runtime | tokio | Industry standard, needed for reqwest |
| Progress bars | indicatif | Best Rust library for terminal progress |
