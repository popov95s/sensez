//! Crate-root detection and path-style module-name keys for Rust.

use crate::profiles::pathroot;
use std::path::{Path, PathBuf};

const MARKERS: [&str; 1] = ["Cargo.toml"];
const BOUNDARIES: [&str; 4] = ["target", ".git", "node_modules", ".venv"];

/// True for a Rust module-index file: `mod.rs`, or the crate root `lib.rs`.
pub fn is_package_index(path: &Path) -> bool {
    path.file_stem().is_some_and(|s| s == "mod" || s == "lib")
}

/// Crate root: nearest ancestor with `Cargo.toml`, else the file's directory.
pub fn root_for(file: &Path) -> PathBuf {
    pathroot::root_for(file, &MARKERS, &BOUNDARIES)
}

/// Root-relative path key: drop `.rs` and collapse a trailing `mod`/`lib`.
pub fn module_name(file: &Path, root: &Path) -> String {
    key_from_parts(pathroot::rel_parts(file, root))
}

/// Apply the extension-strip + `mod`/`lib`-collapse rule to path segments.
pub fn key_from_parts(mut parts: Vec<String>) -> String {
    if let Some(last) = parts.last_mut() {
        if let Some(stripped) = last.strip_suffix(".rs") {
            *last = stripped.to_string();
        }
    }
    if parts.last().is_some_and(|l| l == "mod" || l == "lib") {
        parts.pop();
    }
    parts.join("/")
}

#[cfg(test)]
#[path = "roots_tests.rs"]
mod tests;
