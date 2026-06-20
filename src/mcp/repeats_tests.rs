use super::repeats::{suppress_repeated, suppress_repeated_at, DEFER_EXPIRY_SECS};
use crate::noze::{ActionLevel, AnalysisReport, Severity, SmellFinding, SmellKind};
use std::path::Path;

#[test]
fn suppresses_after_repeat_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("a.py");
    std::fs::write(&file, "def f():\n    pass\n").unwrap();

    for turn in 0..2 {
        let mut report = smell_report(&file, 1);
        let outcome = suppress_repeated(root, &mut report, 2);
        assert_eq!(outcome.deferred, 0, "turn {turn} still reports");
        assert_eq!(report.smells.len(), 1);
    }

    let mut report = smell_report(&file, 1);
    let outcome = suppress_repeated(root, &mut report, 2);
    assert_eq!(outcome.deferred, 1);
    assert!(report.smells.is_empty(), "third repeat is deferred");

    let mut report = smell_report(&file, 1);
    let outcome = suppress_repeated(root, &mut report, 2);
    assert_eq!(outcome.deferred, 1);
    assert!(report.smells.is_empty(), "deferred repeat stays hidden");
}

#[test]
fn line_move_is_a_new_gate_repeat_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("a.py");
    std::fs::write(&file, "def f():\n    pass\n").unwrap();

    let mut first = smell_report(&file, 1);
    suppress_repeated(root, &mut first, 1);
    assert_eq!(first.smells.len(), 1);

    let mut same_line = smell_report(&file, 1);
    suppress_repeated(root, &mut same_line, 1);
    assert!(same_line.smells.is_empty());

    let mut moved = smell_report(&file, 2);
    suppress_repeated(root, &mut moved, 1);
    assert_eq!(moved.smells.len(), 1, "different line gets a fresh count");
}

#[test]
fn first_auto_defer_expires_once_then_second_defer_is_permanent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("a.py");
    std::fs::write(&file, "def f():\n    pass\n").unwrap();

    let mut first = smell_report(&file, 1);
    suppress_repeated_at(root, &mut first, 1, 10);
    assert_eq!(first.smells.len(), 1);

    let mut deferred = smell_report(&file, 1);
    suppress_repeated_at(root, &mut deferred, 1, 11);
    assert!(deferred.smells.is_empty());

    let expiry = 11 + DEFER_EXPIRY_SECS;
    let mut before_expiry = smell_report(&file, 1);
    suppress_repeated_at(root, &mut before_expiry, 1, expiry - 1);
    assert!(before_expiry.smells.is_empty());

    let mut after_expiry = smell_report(&file, 1);
    suppress_repeated_at(root, &mut after_expiry, 1, expiry);
    assert_eq!(
        after_expiry.smells.len(),
        1,
        "first auto-defer expires after three days"
    );

    let mut second_defer = smell_report(&file, 1);
    suppress_repeated_at(root, &mut second_defer, 1, expiry + 1);
    assert!(second_defer.smells.is_empty());

    let mut much_later = smell_report(&file, 1);
    suppress_repeated_at(root, &mut much_later, 1, expiry + DEFER_EXPIRY_SECS * 3);
    assert!(
        much_later.smells.is_empty(),
        "second auto-defer does not get another expiry"
    );
}

fn smell_report(file: &Path, line: usize) -> AnalysisReport {
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
