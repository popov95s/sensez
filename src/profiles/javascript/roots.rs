//! Package-root detection and path-style module-name keys for JS/TS.
//!
//! Unlike Python's dotted packages, a JS module's identity is its file path.
//! We key modules by their root-relative path with the extension dropped and a
//! trailing `index` collapsed (so `src/api/index.js` and an `import "./api"`
//! both resolve to the key `src/api`). Path keys use `/` and cannot collide
//! with Python dotted names in a mixed-language scan.

use crate::profiles::pathroot;
use std::path::{Component, Path, PathBuf};

const MARKERS: [&str; 1] = ["package.json"];
const BOUNDARIES: [&str; 3] = ["node_modules", ".git", ".venv"];
const EXTENSIONS: [&str; 6] = ["ts", "tsx", "js", "jsx", "mjs", "cjs"];

/// True for a JS/TS package index file (`index.js`, `index.ts`, ...).
pub fn is_package_index(path: &Path) -> bool {
    path.file_stem().is_some_and(|s| s == "index")
}

/// Package root: nearest ancestor containing `package.json` (without crossing a
/// vendor boundary), else the file's own directory.
pub fn root_for(file: &Path) -> PathBuf {
    pathroot::root_for(file, &MARKERS, &BOUNDARIES)
}

/// Root-relative path key: drop the extension and collapse a trailing `index`.
/// `root/src/api/index.ts` → `src/api`; `root/src/util.ts` → `src/util`.
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
mod tests {
    use super::*;

    #[test]
    fn module_key_strips_ext_and_index() {
        let root = Path::new("/proj");
        assert_eq!(
            module_name(Path::new("/proj/src/util.ts"), root),
            "src/util"
        );
        assert_eq!(
            module_name(Path::new("/proj/src/api/index.js"), root),
            "src/api"
        );
        assert_eq!(module_name(Path::new("/proj/main.jsx"), root), "main");
    }

    #[test]
    fn normalize_resolves_dot_dot() {
        assert_eq!(
            normalize(Path::new("/proj/src/a/../b/./c")),
            PathBuf::from("/proj/src/b/c")
        );
    }
}
