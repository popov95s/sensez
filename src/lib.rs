#![doc = include_str!("../README.md")]

// Keep the crate root thin: it wires modules together and re-exports the
// intentionally public surface for CLI and library entry points.

mod bonez;
#[cfg(feature = "mcp")]
mod brainz;
mod cli;
pub mod config;
mod config_summary;
mod diff;
mod dotdir;
#[cfg(feature = "eyez")]
mod eyez;
pub mod fingerprints;
mod globs;
#[cfg(feature = "mcp")]
mod mcp;
mod noze;
mod pipeline;
mod profiles;
pub mod report;
mod reporter;
mod setup;
mod spine;

pub use cli::run as run_cli;
pub use pipeline::{analyze_path, scan};
pub use report::*;
pub use reporter::Format;
