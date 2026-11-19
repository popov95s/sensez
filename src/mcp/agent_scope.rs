//! Derive one Claude session's changed lines from its Stop-hook transcript.
//!
//! This runs inside the existing Stop gate call. It never exposes
//! a tracking tool to the model and does not require a second MCP round trip.
//! Edits made by subagents live in their own transcripts and are not seen here.

use crate::diff::ChangedLines;
use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub(super) fn id(session_id: Option<&str>) -> Option<String> {
    session_id.map(|id| format!("session:{id}"))
}

pub(super) fn changes(root: &Path, transcript: &Path) -> Result<ChangedLines> {
    let text = std::fs::read_to_string(transcript)
        .with_context(|| format!("reading Claude transcript {}", transcript.display()))?;
    let mut changed = ChangedLines::default();
    let mut unreadable = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        match serde_json::from_str::<Value>(line) {
            Ok(value) => collect(root, &value, &mut changed),
            Err(_) => unreadable += 1,
        }
    }
    if unreadable > 0 {
        eprintln!(
            "[sensez gate] skipped {unreadable} unparseable line(s) in {}",
            transcript.display()
        );
    }
    Ok(changed)
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
        "MultiEdit" => input
            .get("edits")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_object)
            .for_each(|edit| record_edit(&file, edit, changed)),
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

/// Scope an edit to where its replacement sits now, or to the whole file when that is ambiguous.
fn record_edit(file: &Path, edit: &Map<String, Value>, changed: &mut ChangedLines) {
    if let Some(removed) = edit.get("old_string").and_then(Value::as_str) {
        let identifier_chars =
            crate::profiles::registry::identifier_extra_chars_for_path(Some(file));
        changed.add_removed_text(removed, &identifier_chars);
    }
    let replacement = edit
        .get("new_string")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let replace_all = edit
        .get("replace_all")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let ranges: Vec<(usize, usize)> = if replacement.is_empty() {
        Vec::new()
    } else {
        text.match_indices(replacement)
            .map(|(offset, _)| line_range(&text, offset, replacement))
            .collect()
    };
    if ranges.len() == 1 || (replace_all && !ranges.is_empty()) {
        ranges
            .into_iter()
            .for_each(|(start, end)| changed.add(file, start, end));
    } else {
        changed.add_full_file(file);
    }
}

fn line_range(text: &str, offset: usize, replacement: &str) -> (usize, usize) {
    let start = text[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1;
    let end = start + replacement.bytes().filter(|byte| *byte == b'\n').count();
    (start, end)
}

#[cfg(test)]
#[path = "agent_scope_tests.rs"]
mod tests;
