use serde_json::Value;
use std::path::PathBuf;
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

fn uri_path(uri: &str) -> Option<PathBuf> {
    Url::parse(uri).ok()?.to_file_path().ok()
}
