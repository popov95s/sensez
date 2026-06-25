//! Discovers source files for every compiled-in language under a root directory
//! while honoring `.gitignore` rules via the `ignore` crate.

mod explorer;
mod generated;

pub use explorer::{collect_source_files, Discovery};

use anyhow::Result;
use std::path::Path;

/// Discover source files under `root`, respecting `.gitignore` and skipping
/// any path matching one of the `exclude` globs. Dependency injection allows the caller to provide
/// a language-specific predicate for whether a file is a source file, so that the crawler can be used for any language without having to know about it.
///
/// Unreadable entries are counted in [`Discovery::skipped`], never dropped
/// silently.
pub fn discover<F>(root: &Path, exclude: &[String], is_source_file: &F) -> Result<Discovery>
where
    F: Fn(&Path) -> bool + Send + Sync,
{
    explorer::collect_source_files(root, exclude, is_source_file)
}
