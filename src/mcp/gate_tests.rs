use super::gate::{gate, working_signature};
use serde_json::{json, Value};
use std::process::Command;

#[test]
fn gate_degrades_open() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.to_string_lossy().into_owned();

    let resp = gate(&json!({"path": path, "stop_hook_active": false})).unwrap();
    assert_eq!(resp["content"][0]["text"], "{}", "non-git repo -> allow");

    let resp = gate(&json!({"path": path, "stop_hook_active": "true"})).unwrap();
    assert_eq!(resp["content"][0]["text"], "{}", "second stop -> allow");
}

#[test]
fn signature_tracks_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("a.py");
    std::fs::write(&file, "x = 1\n").unwrap();
    let mut changed = crate::diff::ChangedLines::default();
    changed.add_full_file(&file);

    let sig1 = working_signature(&changed);
    assert_eq!(sig1, working_signature(&changed), "stable when untouched");

    std::fs::write(&file, "x = 1\ny = 2\nz = 3\n").unwrap();
    assert_ne!(sig1, working_signature(&changed), "changes after a write");
}

#[test]
fn gate_baseline_feeds_resolved_recapture() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    if !init_repo(root) {
        return;
    }
    std::fs::write(root.join("added.py"), "def orphan():\n    return 1\n").unwrap();

    let path = root.to_string_lossy().into_owned();
    let resp = gate(&json!({"path": path, "stop_hook_active": false})).unwrap();

    assert_eq!(resp["isError"], false);
    assert!(root.join(".sensez/local-metrics/last-scan.json").exists());

    std::fs::write(root.join("added.py"), "print('fixed')\n").unwrap();
    crate::brainz::recapture();

    let report = crate::brainz::usage_report(root);
    assert_eq!(
        report["all_time"]["resolved_by_detector"]["dead_code/function"]["count"], 1,
        "fixing a gate-reported finding should be counted as resolved"
    );
}

#[test]
fn gate_allows_same_unchanged_work_after_one_block() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    if !init_repo(root) {
        return;
    }
    std::fs::write(root.join("added.py"), "def orphan():\n    return 1\n").unwrap();
    let path = root.to_string_lossy().into_owned();

    let first = gate(&json!({"path": path, "stop_hook_active": false})).unwrap();
    assert_block(&first);

    let second = gate(&json!({"path": path, "stop_hook_active": false})).unwrap();
    assert_eq!(second["content"][0]["text"], "{}");
}

#[test]
fn gate_auto_defers_after_default_repeat_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    if !init_repo(root) {
        return;
    }
    let file = root.join("added.py");
    let path = root.to_string_lossy().into_owned();

    std::fs::write(&file, "def orphan():\n    return 1\n").unwrap();
    assert_block(&gate(&json!({"path": path, "stop_hook_active": false})).unwrap());

    std::fs::write(&file, "def orphan():\n    return 1\n# still here\n").unwrap();
    assert_block(&gate(&json!({"path": path, "stop_hook_active": false})).unwrap());

    std::fs::write(
        &file,
        "def orphan():\n    return 1\n# still here\n# third pass\n",
    )
    .unwrap();
    let third = gate(&json!({"path": path, "stop_hook_active": false})).unwrap();
    assert_eq!(third["content"][0]["text"], "{}");
}

fn assert_block(resp: &Value) {
    let text = resp["content"][0]["text"].as_str().unwrap();
    let decision: Value = serde_json::from_str(text).unwrap();
    assert_eq!(decision["decision"], "block");
    let reason = decision["reason"].as_str().unwrap();
    assert!(reason.contains("sensez gate:"));
    assert!(reason.contains("Top findings:"));
    assert!(!reason.contains("\"meta\""));
    assert!(!reason.contains("Findings (top 5 per pillar)"));
}

fn init_repo(root: &std::path::Path) -> bool {
    if !git(root, &["init"]) {
        return false;
    }
    std::fs::write(root.join("base.py"), "print('base')\n").unwrap();
    git(root, &["add", "."])
        && git(
            root,
            &[
                "-c",
                "user.email=sensez@example.test",
                "-c",
                "user.name=Sensez",
                "commit",
                "-m",
                "base",
            ],
        )
}

fn git(root: &std::path::Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}
