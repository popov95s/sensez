//! Crate-root detection and path-style module-name keys for Rust.
//!
//! Like JS/TS, a Rust module's identity is its file path. We key modules by
//! their root-relative path with the `.rs` extension dropped and a trailing
//! `mod`/`lib` collapsed — so `src/noze/mod.rs` and 2018-style
//! `src/noze.rs` both key as `src/noze`, and `src/lib.rs` (the crate
//! root) keys as `src`, which is exactly what `crate::` resolves against.
//! `main.rs` is NOT collapsed (it is a separate binary crate; collapsing both
//! would collide with `lib.rs`). Path keys use `/` and cannot collide with
//! Python dotted names in a mixed-language scan.

use crate::profiles::pathroot;
use std::path::{Path, PathBuf};

const MARKERS: [&str; 1] = ["Cargo.toml"];
const BOUNDARIES: [&str; 4] = ["target", ".git", "node_modules", ".venv"];

/// True for a Rust module-index file: `mod.rs`, or the crate root `lib.rs`.
pub fn is_package_index(path: &Path) -> bool {
    path.file_stem().is_some_and(|s| s == "mod" || s == "lib")
}

/// Crate root: nearest ancestor containing `Cargo.toml` (without crossing a
/// vendor/build boundary), else the file's own directory.
pub fn root_for(file: &Path) -> PathBuf {
    pathroot::root_for(file, &MARKERS, &BOUNDARIES)
}

/// Root-relative path key: drop `.rs` and collapse a trailing `mod`/`lib`.
/// `root/src/noze/mod.rs` → `src/noze`; `root/src/lib.rs` → `src`.
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
mod tests {
    use super::*;

    #[test]
    fn module_key_strips_ext_and_collapses_mod_and_lib() {
        let root = Path::new("/proj");
        assert_eq!(
            module_name(Path::new("/proj/src/noze/mod.rs"), root),
            "src/noze"
        );
        assert_eq!(
            module_name(Path::new("/proj/src/noze.rs"), root),
            "src/noze"
        );
        assert_eq!(module_name(Path::new("/proj/src/lib.rs"), root), "src");
        assert_eq!(
            module_name(Path::new("/proj/src/main.rs"), root),
            "src/main"
        );
    }
}
