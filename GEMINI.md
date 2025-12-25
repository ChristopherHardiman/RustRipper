# MasterRustRipper Project

## Project Overview

This repository, `MasterRustRipper`, contains a collection of Rust-based tools designed to automate the process of ripping DVDs and Blu-ray discs. The project is divided into four main components: `RustRipper`, `makemkvstarter`, `ripper_starter`, and `OMDb_API`. Together, these tools handle disc detection, metadata fetching, and the ripping process itself.

## Projects

### 1. RustRipper

*   **Purpose:** This tool is responsible for detecting when a new disc is inserted into the drive. It uses `udev` to monitor for device events and can identify the type of disc (e.g., DVD, Blu-ray) and its title.
*   **Key Files:**
    *   `src/main.rs`: The main entry point, containing the disc detection loop.
    *   `src/disc_detect.rs`: Contains the logic for determining the disc type.
    *   `Cargo.toml`: Project manifest, including dependencies like `udev`.

### 2. makemkvstarter

*   **Purpose:** A command-line utility that executes the `makemkv` command-line interface with predefined settings. It reads its configuration from a `conf.toml` file, which specifies the input drive and output directory. If the configuration file is not found, it will prompt the user to create one.
*   **Key Files:**
    *   `src/main.rs`: Contains the logic for reading the configuration, prompting for configuration if it doesn't exist, and executing `makemkv`.
    *   `conf.toml`: The configuration file (will be created on first run if it doesn't exist).
    *   `Cargo.toml`: Project manifest.

### 3. ripper_starter

*   **Purpose:** This tool serves as a setup and initialization utility. It ensures that necessary software, specifically `MakeMKV` and `HandBrake`, are installed as Flatpaks. It also appears to handle some initial configuration setup.
*   **Key Files:**
    *   `src/main.rs`: The main entry point.
    *   `src/flatpak_mod.rs`: Contains the logic for checking and installing Flatpak applications.
    *   `src/config_setup.rs`: Handles the creation of configuration files.
    *   `Cargo.toml`: Project manifest.

### 4. OMDb_API

*   **Purpose:** This utility fetches movie information from the Open Movie Database (OMDb) API. It takes a movie title and an API key (stored in `conf.toml`) and retrieves details about the movie.
*   **Key Files:**
    *   `src/main.rs`: The main entry point.
    *   `src/omdb_api.rs`: Contains the logic for making requests to the OMDb API.
    *   `conf.toml`: Configuration file containing the OMDb API key.
    *   `Cargo.toml`: Project manifest, including dependencies like `reqwest` for making HTTP requests.

## Building and Running

Each of the projects in this repository is a standard Rust project and can be built and run using `cargo`.

To build a specific project, navigate to its directory and run:

```bash
cargo build
```

To run a specific project, navigate to its directory and run:

```bash
cargo run
```

**Note:** Some of the projects require configuration. For example, `makemkvstarter` and `OMDb_API` require a `conf.toml` file. The `ripper_starter` project is designed to help with the initial setup.

## Development Conventions

*   **Language:** All projects are written in Rust (2021 edition).
*   **Dependency Management:** `cargo` is used for dependency management. The dependencies for each project are listed in its `Cargo.toml` file.
*   **Configuration:** The projects use `toml` files for configuration.
*   **Asynchronous Operations:** `tokio` is used for asynchronous operations in `ripper_starter`, `OMDb_API`, and `RustRipper`.
