//! Derive one Claude session's changed lines from its Stop-hook transcript.
//!
//! This runs inside the existing Stop gate call. It never exposes
//! a tracking tool to the model and does not require a second MCP round trip.

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

    #[test]
    fn edit_whose_replacement_also_appears_earlier_scopes_the_whole_file() {
        let (_tmp, root, file) =
            module("def live():\n    return 1\n\n\ndef orphan():\n    return 1\n");
        let edit = json!({"name": "Edit", "input": {"file_path": file, "old_string": "x", "new_string": "    return 1\n"}});
        let changed = changes(&root, &log(&root, &[edit.to_string()])).unwrap();
        assert!(
            changed.touches(&file, 5, 5),
            "the second definition is in scope"
        );
    }

    #[test]
    fn edit_with_a_unique_replacement_scopes_only_its_lines() {
        let (_tmp, root, file) =
            module("def live():\n    return 1\n\n\ndef orphan():\n    return 2\n");
        let edit = json!({"name": "Edit", "input": {"file_path": file, "old_string": "x", "new_string": "    return 2\n"}});
        let changed = changes(&root, &log(&root, &[edit.to_string()])).unwrap();
        assert!(changed.touches(&file, 6, 6));
        assert!(!changed.touches(&file, 1, 2));
    }

    #[test]
    fn replace_all_scopes_every_occurrence() {
        let (_tmp, root, file) = module("a = old\nb = 1\nc = old\n");
        let edit = json!({"name": "Edit", "input": {"file_path": file, "old_string": "x", "new_string": "old", "replace_all": true}});
        let changed = changes(&root, &log(&root, &[edit.to_string()])).unwrap();
        assert!(changed.touches(&file, 1, 1) && changed.touches(&file, 3, 3));
        assert!(!changed.touches(&file, 2, 2));
    }

    #[test]
    fn multi_edit_scopes_each_edit() {
        let (_tmp, root, file) = module("first = 1\nmiddle = 2\nlast = 3\n");
        let edits = json!([
            {"old_string": "x", "new_string": "first = 1"},
            {"old_string": "y", "new_string": "last = 3"}
        ]);
        let edit = json!({"name": "MultiEdit", "input": {"file_path": file, "edits": edits}});
        let changed = changes(&root, &log(&root, &[edit.to_string()])).unwrap();
        assert!(changed.touches(&file, 1, 1) && changed.touches(&file, 3, 3));
        assert!(!changed.touches(&file, 2, 2));
    }

    #[test]
    fn malformed_transcript_line_keeps_the_other_edits() {
        let (_tmp, root, file) = module("x = 1\n");
        let write = json!({"name": "Write", "input": {"file_path": file}});
        let changed = changes(&root, &log(&root, &[write.to_string(), "not json".into()])).unwrap();
        assert!(changed.touches_file(&file));
    }

    fn module(source: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let file = root.join("module.py");
        std::fs::write(&file, source).unwrap();
        (tmp, root, file)
    }

    fn log(root: &Path, lines: &[String]) -> PathBuf {
        let path = root.join("session.jsonl");
        std::fs::write(&path, lines.join("\n")).unwrap();
        path
    }

    fn transcript(root: &Path, name: &str, file: &Path) -> PathBuf {
        let path = root.join(name);
        let line = json!({"name": "Write", "input": {"file_path": file}});
        std::fs::write(&path, line.to_string()).unwrap();
        path
    }
}
