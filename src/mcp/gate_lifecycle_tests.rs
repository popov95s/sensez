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

/// A finding fixed and reintroduced more than `repeat_limit` times defers,
/// and a fresh finding next to it still blocks with the deferral named.
#[test]
fn gate_block_message_mentions_deferred_repeats() {
    let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };
    let orphan_a = "def a():\n    return 1\n\nprint('x')\n";
    let fixed = "print('x')\n";

    for turn in 0..2 {
        std::fs::write(&repo.file, orphan_a).unwrap();
        let decision = block_decision(&gate(&json!({"path": repo.path})).unwrap());
        assert_eq!(decision["decision"], "block", "turn {turn}");
        assert!(
            !decision["reason"]
                .as_str()
                .unwrap()
                .contains("Auto-deferred"),
            "turn {turn}: nothing deferred yet"
        );
        std::fs::write(&repo.file, fixed).unwrap();
        assert_eq!(
            gate(&json!({"path": repo.path})).unwrap()["content"][0]["text"],
            "{}"
        );
    }

    std::fs::write(
        &repo.file,
        "def a():\n    return 1\n\ndef b():\n    return 2\n\nprint('x')\n",
    )
    .unwrap();
    let third = block_decision(&gate(&json!({"path": repo.path})).unwrap());
    assert_eq!(third["decision"], "block");
    let reason = third["reason"].as_str().unwrap();
    assert!(reason.contains("Auto-deferred 1 finding"), "{reason}");
    assert!(reason.contains("1 diff finding(s)"), "{reason}");
}

/// Auto-defer is bounded: the first defer expires after three days
/// and the finding resurfaces; a second defer is permanent.
#[test]
fn gate_deferred_finding_resurfaces_after_expiry() {
    use super::repeats::{suppress_repeated, RepeatState, DEFER_EXPIRY_SECS};
    let file = Path::new("a.py");
    let mut state = RepeatState::default();

    let mut first = smell_report(file, 1);
    let outcome = suppress_repeated(&mut state, &mut first, 1, 10);
    assert_eq!(outcome.deferred, 0);
    assert_eq!(first.smells.len(), 1);

    let mut deferred = smell_report(file, 1);
    let outcome = suppress_repeated(&mut state, &mut deferred, 1, 11);
    assert_eq!(outcome.deferred, 1);
    assert!(deferred.smells.is_empty());

    let expiry = 11 + DEFER_EXPIRY_SECS;
    let mut resurface = smell_report(file, 1);
    let outcome = suppress_repeated(&mut state, &mut resurface, 1, expiry);
    assert_eq!(
        outcome.deferred, 0,
        "expired defer does not count as deferred"
    );
    assert_eq!(
        resurface.smells.len(),
        1,
        "finding resurfaces for re-evaluation"
    );

    let mut second_defer = smell_report(file, 1);
    let outcome = suppress_repeated(&mut state, &mut second_defer, 1, expiry + 1);
    assert_eq!(outcome.deferred, 1);

    let mut much_later = smell_report(file, 1);
    let outcome = suppress_repeated(
        &mut state,
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
            lines: Vec::new(),
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
