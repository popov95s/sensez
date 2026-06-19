//! Rust dead-code conventions. Decorator classification doesn't apply (proc
//! macros are a deferred enhancement), but Cargo gives authoritative entry
//! points: `main.rs`/`build.rs` stems, `[[bin]]` targets, and `src/bin/*.rs`.

use super::roots;
use crate::profiles::{DeadCodeDefaults, DecoratorClass};
use std::collections::HashSet;
use std::path::Path;

pub fn defaults() -> DeadCodeDefaults {
    DeadCodeDefaults {
        test_sources: &["tests/**", "benches/**", "**/*_test.rs"],
        ..DeadCodeDefaults::EMPTY
    }
}

/// Rust has no decorator equivalent in scope yet → never classified.
pub fn classify(_paths: Option<&Vec<String>>, _user: &HashSet<String>) -> DecoratorClass {
    DecoratorClass::None
}

/// A leading underscore marks an intentionally unused binding in Rust.
pub fn is_conventionally_private(symbol: &str) -> bool {
    symbol.starts_with('_')
}

/// Crate entry files Cargo invokes directly (no inbound import edge needed).
pub fn is_entry_file_stem(stem: &str) -> bool {
    matches!(stem, "main" | "build")
}

/// Module keys of binary targets declared in `root/Cargo.toml` (`[[bin]]`
/// `path` entries) plus auto-discovered `src/bin/*.rs`. Best-effort: a
/// missing/invalid manifest contributes none.
pub fn entry_modules(root: &Path) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(manifest) = read_manifest(root) {
        for bin in manifest
            .get("bin")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(path) = bin.get("path").and_then(toml::Value::as_str) {
                keys.push(roots::key_from_parts(
                    path.split('/').map(str::to_string).collect(),
                ));
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(root.join("src/bin")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                keys.push(roots::module_name(&path, root));
            }
        }
    }
    keys
}

fn read_manifest(root: &Path) -> Option<toml::Value> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    text.parse().ok()
}
