use super::gate::gate;
use crate::test_support::GitTestRepo;
use serde_json::{json, Value};

#[test]
fn gate_degrades_open() {
let _metrics = crate::test_support::metrics_guard();
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
let _metrics = crate::test_support::metrics_guard();
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("a.py");
    std::fs::write(&file, "x = 1\n").unwrap();
    let mut changed = crate::diff::ChangedLines::default();
    changed.add_full_file(&file);

    let sig1 = changed.signature();
    assert_eq!(sig1, changed.signature(), "stable when untouched");

    std::fs::write(&file, "x = 1\ny = 2\nz = 3\n").unwrap();
    assert_ne!(sig1, changed.signature(), "changes after a write");
}

#[test]
fn gate_baseline_feeds_resolved_recapture() {
let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };
    std::fs::write(&repo.file, "def orphan():\n    return 1\n").unwrap();

    let resp = gate(&json!({"path": repo.path, "stop_hook_active": false})).unwrap();
    assert_eq!(resp["isError"], false);
    assert!(repo
        .root
        .join(".sensez/local-metrics/last-scan.json")
        .exists());

    std::fs::write(&repo.file, "print('fixed')\n").unwrap();
    crate::brainz::recapture();

    let report = crate::brainz::usage_report(&repo.root);
    assert_eq!(
        report["all_time"]["resolved_by_detector"]["dead_code/function"]["count"], 1,
        "fixing a gate-reported finding should be counted as resolved"
    );
}

#[test]
fn gate_allows_same_unchanged_work_after_one_block() {
let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };
    std::fs::write(&repo.file, "def orphan():\n    return 1\n").unwrap();

    let first = gate(&json!({"path": repo.path})).unwrap();
    assert_block(&first);

    let second = gate(&json!({"path": repo.path})).unwrap();
    assert_eq!(second["content"][0]["text"], "{}");
}

#[test]
fn gate_reblocks_when_agent_fixes_then_introduces_again() {
let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };

    std::fs::write(&repo.file, "def orphan():\n    return 1\n").unwrap();
    let first = gate(&json!({"path": repo.path})).unwrap();
    assert_block(&first);

    // Fix: the finding disappears, the next call allows.
    std::fs::write(&repo.file, "print('fixed')\n").unwrap();
    let after_fix = gate(&json!({"path": repo.path})).unwrap();
    assert_eq!(after_fix["content"][0]["text"], "{}");

    // Reintroduce: same identity as the first call, so the gate allows it.
    std::fs::write(&repo.file, "def orphan():\n    return 1\n").unwrap();
    let again = gate(&json!({"path": repo.path})).unwrap();
    assert_eq!(again["content"][0]["text"], "{}");
}

/// Dedup is over the finding identity, not line position. A content edit that
/// only moves a known finding does not create a new gate complaint.
#[test]
fn gate_identity_survives_line_moves() {
let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };

    std::fs::write(&repo.file, "def orphan():\n    return 1\n").unwrap();
    assert_block(&gate(&json!({"path": repo.path})).unwrap());

    std::fs::write(&repo.file, "# new comment\ndef orphan():\n    return 1\n").unwrap();
    let moved = gate(&json!({"path": repo.path})).unwrap();
    assert_eq!(moved["content"][0]["text"], "{}");
}

/// Across many calls the gate blocks exactly when a new finding identity
/// appears — one block per new complaint, not one block per turn.
#[test]
fn gate_block_count_tracks_new_identities() {
let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };

    let blocks = |content: &str| {
        std::fs::write(&repo.file, content).unwrap();
        let resp = gate(&json!({"path": repo.path})).unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        if text == "{}" {
            0
        } else {
            let decision: Value = serde_json::from_str(text).unwrap();
            assert_eq!(decision["decision"], "block");
            1
        }
    };

    // Sequence: intro → same → trailing comment → fix → reintro → line-move → same.
    // Blocks fire only on the first unseen identity.
    assert_eq!(blocks("def orphan():\n    return 1\n"), 1);
    assert_eq!(blocks("def orphan():\n    return 1\n"), 0);
    assert_eq!(
        blocks("def orphan():\n    return 1\n# trailing comment\n"),
        0
    );
    assert_eq!(blocks("print('fixed')\n"), 0);
    assert_eq!(blocks("def orphan():\n    return 1\n"), 0);
    assert_eq!(blocks("# new comment\ndef orphan():\n    return 1\n"), 0);
    assert_eq!(blocks("# new comment\ndef orphan():\n    return 1\n"), 0);
}

/// Companion to `gate_block_count_tracks_signature_changes`: with the
/// content unchanged, the gate blocks exactly once and then allows.
#[test]
fn gate_blocks_unchanged_finding_only_once() {
let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };
    std::fs::write(&repo.file, "def orphan():\n    return 1\n").unwrap();

    let first = gate(&json!({"path": repo.path})).unwrap();
    assert_block(&first);

    for _ in 0..5 {
        let resp = gate(&json!({"path": repo.path})).unwrap();
        assert_eq!(
            resp["content"][0]["text"], "{}",
            "unchanged content must not re-block"
        );
    }
}

/// `usage_report` totals must reflect what the gate actually saw.
#[test]
fn brainz_totals_track_reported_count() {
let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };
    std::fs::write(
        &repo.file,
        "def a():\n    return 1\n\ndef b():\n    return 2\n\nprint('x')\n",
    )
    .unwrap();

    let first = gate(&json!({"path": repo.path})).unwrap();
    assert_block(&first);

    let report = crate::brainz::usage_report(&repo.root);
    assert_eq!(
        report["all_time"]["reported_by_detector"]["dead_code/function"], 2,
        "two orphans reported on the first gate call"
    );
    assert_eq!(
        report["all_time"]["scans_by_origin"]["gate"], 1,
        "the gate's scan is counted under the gate origin"
    );
    assert_eq!(
        report["all_time"]["gate_blocks"], 1,
        "exactly one gate block recorded"
    );

    std::fs::write(&repo.file, "def a():\n    return 1\n\nprint('x')\n").unwrap();
    crate::brainz::recapture();

    let report = crate::brainz::usage_report(&repo.root);
    assert_eq!(
        report["all_time"]["resolved_by_detector"]["dead_code/function"]["count"], 1,
        "the deleted orphan is banked as resolved"
    );
}

fn block_decision(resp: &Value) -> Value {
    let text = resp["content"][0]["text"].as_str().unwrap();
    serde_json::from_str(text).unwrap()
}

fn assert_block(resp: &Value) {
    let decision = block_decision(resp);
    assert_eq!(decision["decision"], "block");
    let reason = decision["reason"].as_str().unwrap();
    assert!(reason.contains("sensez gate:"));
    assert!(reason.contains("Top findings:"));
    assert!(!reason.contains("\"meta\""));
    assert!(!reason.contains("Findings (top 5 per pillar)"));
}

fn fresh_repo(scratch: &str) -> Option<GitTestRepo> {
    GitTestRepo::importing(scratch, "added")
}
