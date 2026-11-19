//! `Cargo.toml` reads shared by the Rust resolver and dead-code entry points.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

type Cached = (Option<SystemTime>, Option<Arc<toml::Value>>);

/// The parsed `root/Cargo.toml`, memoized per root until the file's mtime changes.
pub fn manifest(root: &Path) -> Option<Arc<toml::Value>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Cached>>> = OnceLock::new();
    let path = root.join("Cargo.toml");
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    let mut cache = match CACHE.get_or_init(Mutex::default).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some((stamp, value)) = cache.get(root) {
        if *stamp == modified {
            return value.clone();
        }
    }
    let value = read(&path).map(Arc::new);
    cache.insert(root.to_path_buf(), (modified, value.clone()));
    value
}

fn read(path: &Path) -> Option<toml::Value> {
    std::fs::read_to_string(path).ok()?.parse().ok()
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
