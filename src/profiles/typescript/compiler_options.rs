//! `compilerOptions` of a `tsconfig.json` merged over its relative `extends` chain.

use super::tsconfig::read_tsconfig;
use serde_json::{Map, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const MAX_EXTENDS_DEPTH: usize = 16;

/// Merged compiler options with `baseUrl` made absolute.
#[derive(Default)]
pub(super) struct CompilerOptions {
    pub values: Map<String, Value>,
    /// Directory of the config that declared `paths`.
    pub paths_dir: Option<PathBuf>,
}

impl CompilerOptions {
    pub(super) fn load(config: &Path) -> Self {
        Self::load_at_depth(config, 0)
    }

    /// Directory `paths` targets are relative to: `baseUrl`, else the declaring config.
    pub(super) fn paths_base(&self) -> Option<PathBuf> {
        self.values
            .get("baseUrl")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .or_else(|| self.paths_dir.clone())
    }

    fn load_at_depth(config: &Path, depth: usize) -> Self {
        let Some(value) = read_tsconfig(config) else {
            return Self::default();
        };
        let dir = config.parent().unwrap_or(Path::new(""));
        let mut merged = Self::default();
        if depth < MAX_EXTENDS_DEPTH {
            for base in extends(&value) {
                merged.overlay(Self::load_at_depth(&base_config(dir, base), depth + 1));
            }
        }
        if let Some(own) = value.get("compilerOptions").and_then(Value::as_object) {
            merged.overlay(Self::own(own, dir));
        }
        merged
    }

    fn own(options: &Map<String, Value>, dir: &Path) -> Self {
        let mut values = options.clone();
        if let Some(base_url) = values.get_mut("baseUrl") {
            if let Some(relative) = base_url.as_str() {
                *base_url = Value::String(dir.join(relative).to_string_lossy().into_owned());
            }
        }
        Self {
            paths_dir: values.contains_key("paths").then(|| dir.to_path_buf()),
            values,
        }
    }

    fn overlay(&mut self, later: Self) {
        self.values.extend(later.values);
        if later.paths_dir.is_some() {
            self.paths_dir = later.paths_dir;
        }
    }
}

/// Relative `extends` entries; package bases fall back to TypeScript defaults.
fn extends(value: &Value) -> Vec<&str> {
    let entries = match value.get("extends") {
        Some(Value::String(base)) => vec![base.as_str()],
        Some(Value::Array(bases)) => bases.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    entries
        .into_iter()
        .filter(|base| base.starts_with('.'))
        .collect()
}

fn base_config(dir: &Path, base: &str) -> PathBuf {
    let path = dir.join(base);
    if path.is_file() {
        return path;
    }
    let mut with_json = OsString::from(path);
    with_json.push(".json");
    PathBuf::from(with_json)
}
