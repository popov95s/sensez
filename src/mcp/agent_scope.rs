//! Derive one Claude session's changed lines from its Stop-hook transcript.
//!
//! This runs inside the existing Stop gate call. It never exposes
//! a tracking tool to the model and does not require a second MCP round trip.

use crate::diff::ChangedLines;
use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub(super) fn id(session_id: Option<&str>) -> Option<String> {
    useful(session_id).map(|id| format!("session:{id}"))
}

pub(super) fn changes(root: &Path, transcript: &Path) -> Result<ChangedLines> {
    let text = std::fs::read_to_string(transcript)
        .with_context(|| format!("reading Claude transcript {}", transcript.display()))?;
    let mut changed = ChangedLines::default();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(line).context("parsing Claude transcript JSONL")?;
        collect(root, &value, &mut changed);
    }
    Ok(changed)
}

fn useful(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty() && !value.starts_with("${"))
}

fn collect(root: &Path, value: &Value, changed: &mut ChangedLines) {
    match value {
        Value::Array(values) => values
            .iter()
            .for_each(|value| collect(root, value, changed)),
        Value::Object(object) => {
            record(root, object, changed);
            object
                .values()
                .for_each(|value| collect(root, value, changed));
        }
        _ => {}
    }
}

fn record(root: &Path, object: &Map<String, Value>, changed: &mut ChangedLines) {
    let name = object
        .get("name")
        .or_else(|| object.get("tool_name"))
        .and_then(Value::as_str);
    let input = object
        .get("input")
        .or_else(|| object.get("tool_input"))
        .and_then(Value::as_object);
    let (Some(name), Some(input)) = (name, input) else {
        return;
    };
    let Some(raw_path) = input.get("file_path").and_then(Value::as_str) else {
        return;
    };
    let file = resolve(root, raw_path);
    match name {
        "Write" => changed.add_full_file(&file),
        "Edit" => record_edit(&file, input, changed),
        _ => {}
    }
}

fn resolve(root: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn record_edit(file: &Path, input: &Map<String, Value>, changed: &mut ChangedLines) {
    let replacement = input
        .get("new_string")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if replacement.is_empty() {
        // A deletion has no replacement span to locate after the edit. Scan
        // the file rather than silently omitting a change that can alter a
        // structural finding.
        changed.add_full_file(file);
        return;
    }
    let range = std::fs::read_to_string(file)
        .ok()
        .and_then(|text| line_range(&text, replacement));
    match range {
        Some((start, end)) => changed.add(file, start, end),
        None => changed.add_full_file(file),
    }
}

fn line_range(text: &str, replacement: &str) -> Option<(usize, usize)> {
    let offset = text.find(replacement)?;
    let start = text[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1;
    let end = start + replacement.bytes().filter(|byte| *byte == b'\n').count();
    Some((start, end.max(start)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn separate_session_transcripts_keep_write_scopes_separate() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("session_a.py");
        let b = root.join("session_b.py");
        std::fs::write(&a, "def session_a():\n    return 1\n").unwrap();
        std::fs::write(&b, "def session_b():\n    return 2\n").unwrap();
        let a_log = transcript(root, "a.jsonl", &a);
        let b_log = transcript(root, "b.jsonl", &b);

        let changed_a = changes(root, &a_log).unwrap();
        let changed_b = changes(root, &b_log).unwrap();
        assert!(changed_a.touches_file(&a) && !changed_a.touches_file(&b));
        assert!(changed_b.touches_file(&b) && !changed_b.touches_file(&a));
    }

    #[test]
    fn session_id_is_the_scope_identity() {
        assert_eq!(
            id(Some("session-123")).as_deref(),
            Some("session:session-123")
        );
    }

    fn transcript(root: &Path, name: &str, file: &Path) -> PathBuf {
        let path = root.join(name);
        let line = json!({"name": "Write", "input": {"file_path": file}});
        std::fs::write(&path, line.to_string()).unwrap();
        path
    }
}
