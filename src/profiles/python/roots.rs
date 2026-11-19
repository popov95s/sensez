//! Package-root detection and dotted module-name computation.

use std::path::{Path, PathBuf};

const MARKERS: [&str; 3] = ["pyproject.toml", "setup.py", "setup.cfg"];
/// Boundaries the marker walk must not cross.
const BOUNDARIES: [&str; 6] = [
    "site-packages",
    ".venv",
    "venv",
    "node_modules",
    ".tox",
    ".git",
];

/// True for a Python package index file (`__init__.py`).
pub fn is_package_index(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == "__init__.py")
}

/// Determine the package root for a single file.
pub fn root_for(file: &Path) -> PathBuf {
    let start = file.parent().unwrap_or(Path::new("."));
    if let Some(marker) = nearest_marker(start) {
        let src = marker.join("src");
        if src.is_dir() && file.starts_with(&src) {
            return src; // src-layout: modules are named relative to src/
        }
        return marker;
    }
    init_walk(start)
}

/// Nearest ancestor directory containing a project marker, not crossing a vendor boundary.
fn nearest_marker(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        if d.file_name()
            .is_some_and(|n| BOUNDARIES.contains(&n.to_string_lossy().as_ref()))
        {
            break; // don't escape into a parent project past this boundary
        }
        if MARKERS.iter().any(|m| d.join(m).exists()) {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    None
}

/// Walk up while each directory has `__init__.py`.
fn init_walk(start: &Path) -> PathBuf {
    let mut dir = start.to_path_buf();
    while dir.join("__init__.py").exists() {
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break,
        }
    }
    dir
}

/// Compute a file's dotted module name relative to its root.
pub fn module_name(file: &Path, root: &Path) -> String {
    let rel = file.strip_prefix(root).unwrap_or(file);
    let mut parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();

    if let Some(last) = parts.last_mut() {
        if last == "__init__.py" {
            parts.pop();
        } else if let Some(stripped) = last.strip_suffix(".py") {
            *last = stripped.to_string();
        }
    }
    parts.join(".")
}

#[cfg(test)]
#[path = "roots_tests.rs"]
mod tests;
