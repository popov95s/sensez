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
    let (_tmp, root, file) = module("def live():\n    return 1\n\n\ndef orphan():\n    return 1\n");
    let edit = json!({"name": "Edit", "input": {"file_path": file, "old_string": "x", "new_string": "    return 1\n"}});
    let changed = changes(&root, &log(&root, &[edit.to_string()])).unwrap();
    assert!(
        changed.touches(&file, 5, 5),
        "the second definition is in scope"
    );
}

#[test]
fn edit_with_a_unique_replacement_scopes_only_its_lines() {
    let (_tmp, root, file) = module("def live():\n    return 1\n\n\ndef orphan():\n    return 2\n");
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
