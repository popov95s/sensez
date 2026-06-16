//! Discovers source files for every compiled-in language under a root directory
//! while honoring `.gitignore` rules via the `ignore` crate.

mod explorer;
mod generated;

pub use explorer::Discovery;

use anyhow::Result;
use std::path::Path;

/// Discover all supported-language source files under `root`, respecting
/// `.gitignore` and skipping any path matching one of the `exclude` globs.
/// Unreadable entries are counted in [`Discovery::skipped`], never dropped
/// silently.
pub fn discover(root: &Path, exclude: &[String]) -> Result<Discovery> {
    explorer::collect_source_files(root, exclude)
}
