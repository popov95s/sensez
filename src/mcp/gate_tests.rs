use super::gate::gate;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
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

    let sig1 = changed.signature();
    assert_eq!(sig1, changed.signature(), "stable when untouched");

    std::fs::write(&file, "x = 1\ny = 2\nz = 3\n").unwrap();
    assert_ne!(sig1, changed.signature(), "changes after a write");
}

#[test]
fn gate_baseline_feeds_resolved_recapture() {
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

/// Fix-then-reintroduce: recapture banks the fix as resolved, then
/// counts the reappearance as a reintroduction.
#[test]
fn brainz_records_fix_and_reintroduction() {
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };

    std::fs::write(&repo.file, "def orphan():\n    return 1\n").unwrap();
    let first = gate(&json!({"path": repo.path})).unwrap();
    assert_block(&first);

    std::fs::write(&repo.file, "print('fixed')\n").unwrap();
    crate::brainz::recapture();

    let report = crate::brainz::usage_report(&repo.root);
    assert_eq!(
        report["all_time"]["resolved_by_detector"]["dead_code/function"]["count"], 1,
        "fix is recorded as resolved"
    );
    assert_eq!(
        report["all_time"]["reintroduced_by_detector"]["dead_code/function"].get("count"),
        None,
        "no reintroduction yet"
    );

    std::fs::write(&repo.file, "def orphan():\n    return 1\n").unwrap();
    crate::brainz::recapture();

    let report = crate::brainz::usage_report(&repo.root);
    assert_eq!(
        report["all_time"]["reintroduced_by_detector"]["dead_code/function"]["count"], 1,
        "reintroduction is recorded"
    );
}

/// When a past-limit finding defers and a fresh one stays, the gate's
/// block reason names the deferred count.
#[test]
fn gate_block_message_mentions_deferred_repeats() {
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };

    // Turn 1: one orphan, fresh — block, no deferral.
    std::fs::write(&repo.file, "def a():\n    return 1\n\nprint('x')\n").unwrap();
    let first_decision = block_decision(&gate(&json!({"path": repo.path})).unwrap());
    assert_eq!(first_decision["decision"], "block");
    assert!(
        !first_decision["reason"]
            .as_str()
            .unwrap()
            .contains("Auto-deferred"),
        "first turn: nothing deferred yet"
    );

    // Turn 2: same content — signature dedup, allow.
    let second = gate(&json!({"path": repo.path})).unwrap();
    assert_eq!(second["content"][0]["text"], "{}");

    // Turn 3: `a` is past repeat_limit and defers; `b` is fresh and
    // stays. Report signature differs from the last blocked — block
    // with the deferred count named in the reason.
    std::fs::write(
        &repo.file,
        "def a():\n    return 1\n\ndef b():\n    return 2\n\nprint('x')\n",
    )
    .unwrap();
    let third_decision = block_decision(&gate(&json!({"path": repo.path})).unwrap());
    assert_eq!(third_decision["decision"], "block");
    assert!(
        third_decision["reason"]
            .as_str()
            .unwrap()
            .contains("Auto-deferred 1 finding"),
        "deferred count is named in the block reason: {}",
        third_decision["reason"]
    );
}

/// Auto-defer is bounded: the first defer expires after three days
/// and the finding resurfaces; a second defer is permanent.
#[test]
fn gate_deferred_finding_resurfaces_after_expiry() {
    use super::repeats::{suppress_repeated_at, DEFER_EXPIRY_SECS};
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("a.py");
    std::fs::write(&file, "def f():\n    pass\n").unwrap();

    let mut first = smell_report(&file, 1);
    let outcome = suppress_repeated_at(root, &mut first, 1, 10);
    assert_eq!(outcome.deferred, 0);
    assert_eq!(first.smells.len(), 1);

    let mut deferred = smell_report(&file, 1);
    let outcome = suppress_repeated_at(root, &mut deferred, 1, 11);
    assert_eq!(outcome.deferred, 1);
    assert!(deferred.smells.is_empty());

    let expiry = 11 + DEFER_EXPIRY_SECS;
    let mut resurface = smell_report(&file, 1);
    let outcome = suppress_repeated_at(root, &mut resurface, 1, expiry);
    assert_eq!(
        outcome.deferred, 0,
        "expired defer does not count as deferred"
    );
    assert_eq!(
        resurface.smells.len(),
        1,
        "finding resurfaces for re-evaluation"
    );

    let mut second_defer = smell_report(&file, 1);
    let outcome = suppress_repeated_at(root, &mut second_defer, 1, expiry + 1);
    assert_eq!(outcome.deferred, 1);

    let mut much_later = smell_report(&file, 1);
    let outcome = suppress_repeated_at(root, &mut much_later, 1, expiry + DEFER_EXPIRY_SECS * 10);
    assert_eq!(outcome.deferred, 1, "second defer is permanent");
    assert!(much_later.smells.is_empty());
}

fn smell_report(file: &Path, line: usize) -> crate::report::AnalysisReport {
    use crate::report::{ActionLevel, AnalysisReport, Severity, SmellFinding, SmellKind};
    AnalysisReport {
        smells: vec![SmellFinding {
            action: ActionLevel::Advisory,
            kind: SmellKind::LongFunction,
            message: "long".to_string(),
            file: file.to_path_buf(),
            line,
            end_line: line + 4,
            symbol: "f".to_string(),
            severity: Severity::Warning,
            metric: 12,
            threshold: 10,
            reason: String::new(),
        }],
        ..AnalysisReport::default()
    }
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

/// Fresh git repo with one initial commit and a scratch file. Owns
/// the `TempDir` so the directory stays alive for the test body.
struct TestRepo {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    file: PathBuf,
    path: String,
}

fn fresh_repo(scratch: &str) -> Option<TestRepo> {
    let tmp = tempfile::tempdir().ok()?;
    let root = tmp.path().to_path_buf();
    if !init_repo(&root) {
        return None;
    }
    Some(TestRepo {
        _tmp: tmp,
        file: root.join(scratch),
        path: root.to_string_lossy().into_owned(),
        root,
    })
}

fn init_repo(root: &Path) -> bool {
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

fn git(root: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}
