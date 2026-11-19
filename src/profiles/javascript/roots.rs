//! Package-root detection and path-style module-name keys for JS/TS.

use crate::profiles::pathroot;
use std::path::{Component, Path, PathBuf};

const MARKERS: [&str; 1] = ["package.json"];
const BOUNDARIES: [&str; 3] = ["node_modules", ".git", ".venv"];
const EXTENSIONS: [&str; 8] = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

/// True for a JS/TS package index file (`index.js`, `index.ts`, ...).
pub fn is_package_index(path: &Path) -> bool {
    path.file_stem().is_some_and(|s| s == "index")
}

/// Package root: nearest ancestor with `package.json`, else the file's directory.
pub fn root_for(file: &Path) -> PathBuf {
    pathroot::root_for(file, &MARKERS, &BOUNDARIES)
}

/// Nearest directory at or above `start` holding a `package.json`.
pub fn workspace_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|directory| directory.join("package.json").is_file())
        .map(Path::to_path_buf)
}

/// Root-relative path key: drop the extension and collapse a trailing `index`.
pub fn module_name(file: &Path, root: &Path) -> String {
    key_from_parts(pathroot::rel_parts(file, root))
}

/// Apply the extension-strip + `index`-collapse rule to path segments.
pub fn key_from_parts(mut parts: Vec<String>) -> String {
    if let Some(last) = parts.last_mut() {
        for ext in EXTENSIONS {
            if let Some(stripped) = last.strip_suffix(&format!(".{ext}")) {
                *last = stripped.to_string();
                break;
            }
        }
    }
    if parts.last().is_some_and(|l| l == "index") {
        parts.pop();
    }
    parts.join("/")
}

/// `path` ends in a JS/TS source extension.
pub fn has_source_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| EXTENSIONS.iter().any(|known| ext == *known))
}

/// Extension-less, `index`-collapsed path naming the module at `path`.
pub fn path_key(path: &Path) -> PathBuf {
    let mut key = normalize(path);
    if has_source_extension(&key) {
        key.set_extension("");
    }
    if key.file_name().is_some_and(|name| name == "index") {
        key.pop();
    }
    key
}

/// Normalize a path's `.`/`..` components without touching the filesystem.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                if matches!(out.last(), Some(Component::Normal(_))) {
                    out.pop();
                } else {
                    out.push(comp);
                }
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out.iter().collect()
}

#[cfg(test)]
#[path = "roots_tests.rs"]
mod tests;
