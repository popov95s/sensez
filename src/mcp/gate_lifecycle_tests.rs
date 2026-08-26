use super::gate::gate;
use crate::test_support::GitTestRepo;
use serde_json::{json, Value};
use std::path::Path;

/// Fix-then-reintroduce: recapture banks the fix as resolved, then
/// counts the reappearance as a reintroduction.
#[test]
fn brainz_records_fix_and_reintroduction() {
    let _metrics = crate::test_support::metrics_guard();
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
    let _metrics = crate::test_support::metrics_guard();
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
    let _metrics = crate::test_support::metrics_guard();
    use super::repeats::{suppress_repeated_at, DEFER_EXPIRY_SECS};
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("a.py");
    std::fs::write(&file, "def f():\n    pass\n").unwrap();

    let mut first = smell_report(&file, 1);
    let outcome = suppress_repeated_at(root, None, &mut first, 1, 10);
    assert_eq!(outcome.deferred, 0);
    assert_eq!(first.smells.len(), 1);

    let mut deferred = smell_report(&file, 1);
    let outcome = suppress_repeated_at(root, None, &mut deferred, 1, 11);
    assert_eq!(outcome.deferred, 1);
    assert!(deferred.smells.is_empty());

    let expiry = 11 + DEFER_EXPIRY_SECS;
    let mut resurface = smell_report(&file, 1);
    let outcome = suppress_repeated_at(root, None, &mut resurface, 1, expiry);
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
    let outcome = suppress_repeated_at(root, None, &mut second_defer, 1, expiry + 1);
    assert_eq!(outcome.deferred, 1);

    let mut much_later = smell_report(&file, 1);
    let outcome = suppress_repeated_at(
        root,
        None,
        &mut much_later,
        1,
        expiry + DEFER_EXPIRY_SECS * 10,
    );
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

fn fresh_repo(scratch: &str) -> Option<GitTestRepo> {
    GitTestRepo::importing(scratch, "added")
}
