//! A [`ChangedLines`] set (built from a unified diff or from git) records which
//! lines of which files a change touched. [`filter`] then keeps only the
//! findings whose line span intersects the change, and attaches provenance.
//! Pure parsing/filtering lives here and in `parse`/`filter`; the only
//! subprocess use (shelling to `git`) is isolated in `git`.

mod filter;
pub mod git;
mod parse;

pub use filter::apply;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Added/modified line ranges per file (keyed by canonicalized absolute path).
#[derive(Debug, Default)]
pub struct ChangedLines {
    files: HashMap<PathBuf, Vec<(usize, usize)>>,
}

impl ChangedLines {
    /// Build from unified-diff text. `base` resolves the diff's (relative)
    /// paths to absolute paths so they can be matched against finding paths.
    pub fn from_unified(text: &str, base: &Path) -> Self {
        let mut changed = ChangedLines::default();
        for (rel, ranges) in parse::parse_unified(text) {
            let path = base.join(rel);
            for (lo, hi) in ranges {
                changed.add(&path, lo, hi);
            }
        }
        changed
    }

    /// Record a changed range `[lo, hi]` (1-indexed) for `file`.
    pub fn add(&mut self, file: &Path, lo: usize, hi: usize) {
        self.files.entry(canon(file)).or_default().push((lo, hi));
    }

    /// Mark an entire file as changed (e.g. a freshly-added/untracked file).
    pub fn add_full_file(&mut self, file: &Path) {
        self.add(file, 1, usize::MAX);
    }

    /// True if `[lo, hi]` overlaps any changed range in `file`.
    pub fn touches(&self, file: &Path, lo: usize, hi: usize) -> bool {
        self.files
            .get(&canon(file))
            .is_some_and(|ranges| ranges.iter().any(|&(a, b)| lo <= b && a <= hi))
    }

    /// True if `file` has any changed range at all.
    pub fn touches_file(&self, file: &Path) -> bool {
        self.files.contains_key(&canon(file))
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// The (canonicalized) paths with any recorded change.
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.files.keys().map(PathBuf::as_path)
    }
}

/// Canonicalize when possible so relative/absolute paths compare equal; fall
/// back to the path as-given (e.g. for paths that don't exist on disk).
fn canon(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
