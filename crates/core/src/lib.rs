//! Core types and error handling for RustRipper
//!
//! This crate provides shared types, error definitions, and configurations
//! used across all other RustRipper crates.

pub mod error;
pub mod types;
pub mod config;

pub use error::{RipperError, Result};
pub use types::*;
pub use config::Config;
