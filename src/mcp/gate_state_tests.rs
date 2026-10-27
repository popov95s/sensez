use super::*;
use crate::report::{ActionLevel, Confidence, DeadCodeFinding};
use crate::spine::ir::SymbolKind;
use std::path::PathBuf;

#[test]
fn blocked_findings_are_isolated_by_session_scope() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    update(root, Some("session:one"), 10, |state| {
        state.remember(&report_with(&["alpha"]))
    });

    let mut other_session = report_with(&["alpha"]);
    update(root, Some("session:two"), 11, |state| {
        state.recall(&mut other_session)
    });
    assert_eq!(other_session.finding_count(), 1);

    let mut original_session = report_with(&["alpha"]);
    update(root, Some("session:one"), 12, |state| {
        state.recall(&mut original_session)
    });
    assert_eq!(original_session.finding_count(), 0);
}

#[test]
fn fixed_then_reintroduced_finding_is_kept_and_reported_back() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    update(root, None, 10, |state| {
        state.remember(&report_with(&["alpha"]))
    });
    update(root, None, 11, |state| state.recall(&mut report_with(&[])));

    let mut again = report_with(&["alpha"]);
    let reintroduced = update(root, None, 12, |state| state.recall(&mut again));
    assert_eq!(again.finding_count(), 1);
    assert_eq!(reintroduced.len(), 1);
}

#[test]
fn idle_scope_expires_after_the_ttl() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    update(root, None, 10, |state| {
        state.remember(&report_with(&["alpha"]))
    });

    let mut later = report_with(&["alpha"]);
    update(root, None, 10 + SCOPE_TTL_SECS, |state| {
        state.recall(&mut later)
    });
    assert_eq!(later.finding_count(), 1);
}

fn report_with(symbols: &[&str]) -> AnalysisReport {
    AnalysisReport {
        dead_code: symbols
            .iter()
            .map(|symbol| DeadCodeFinding {
                action: ActionLevel::Advisory,
                module: "sample".to_string(),
                symbol: (*symbol).to_string(),
                kind: SymbolKind::Function,
                confidence: Confidence::High,
                file: PathBuf::from("sample.py"),
                line: 4,
                reason: String::new(),
            })
            .collect(),
        ..AnalysisReport::default()
    }
}
