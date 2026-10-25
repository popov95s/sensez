use serde_json::Value;
use std::path::{Path, PathBuf};
use url::Url;

pub fn roots(params: &Value) -> Vec<PathBuf> {
    let folders = params
        .get("workspaceFolders")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    let mut roots: Vec<_> = folders
        .filter_map(|folder| folder.get("uri").and_then(Value::as_str))
        .filter_map(uri_path)
        .collect();
    if roots.is_empty() {
        roots.extend(
            params
                .get("rootUri")
                .and_then(Value::as_str)
                .and_then(uri_path),
        );
    }
    roots
}

pub fn notification_path(params: &Value) -> Option<PathBuf> {
    params
        .pointer("/textDocument/uri")
        .and_then(Value::as_str)
        .and_then(uri_path)
}

/// Paths in a `workspace/didChangeWatchedFiles` notification.
pub fn changed_paths(params: &Value) -> Vec<PathBuf> {
    params
        .get("changes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|change| change.get("uri").and_then(Value::as_str))
        .filter_map(uri_path)
        .collect()
}

/// Whether a change to `path` can alter findings: a parsed source or a config file.
pub fn affects_scan(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    crate::profiles::registry::parse_for_path(path).is_some()
        || matches!(
            name,
            "sensez.toml" | "pyproject.toml" | "package.json" | "Cargo.toml"
        )
        || (name.starts_with("tsconfig") && name.ends_with(".json"))
}

fn uri_path(uri: &str) -> Option<PathBuf> {
    Url::parse(uri).ok()?.to_file_path().ok()
}
