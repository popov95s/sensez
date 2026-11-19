use super::*;
use crate::report::{ActionLevel, Severity, SmellFinding, SmellKind};

#[test]
fn diff_reports_git_errors_instead_of_returning_a_full_scan() {
    let tmp = tempfile::tempdir().unwrap();
    let err = run(tmp.path(), None, 0, Scope::DiffVsHead).unwrap_err();
    assert!(err.to_string().contains("resolving working-tree diff"));
}

#[test]
fn only_skipped_analyzer_issues_reach_clients() {
    let issue = |stage| crate::report::ScanIssue {
        stage,
        file: None,
        message: String::new(),
    };
    let mut report = AnalysisReport::default();
    report.meta.issues = vec![
        issue(crate::report::ScanStage::Parse),
        issue(crate::report::ScanStage::Analyze),
    ];

    suppress_scan_issues(&mut report);

    assert_eq!(report.meta.issues.len(), 1);
    assert_eq!(
        report.meta.issues[0].stage,
        crate::report::ScanStage::Analyze
    );
}

#[test]
fn diff_keeps_changed_findings_beyond_the_full_report_limit() {
    let (mut report, file) = three_smells();
    let mut changed = crate::diff::ChangedLines::default();
    changed.add(&file, 3, 3);

    finish_diff(&mut report, &changed, &HashMap::new(), 1);

    assert_eq!(report.smells.len(), 1);
    assert_eq!(report.smells[0].line, 3);
}

#[test]
fn limited_diff_report_keeps_pre_limit_totals() {
    let (mut report, file) = three_smells();
    let mut changed = crate::diff::ChangedLines::default();
    changed.add(&file, 1, 3);

    finish_diff(&mut report, &changed, &HashMap::new(), 1);
    let compact = super::super::compact::tool_report(report);
    let meta = serde_json::to_value(compact).unwrap()["meta"].clone();

    assert_eq!(meta["totals"]["smells"], 3);
    assert_eq!(meta["truncated"], true);
}

fn three_smells() -> (AnalysisReport, PathBuf) {
    let file = PathBuf::from("changed.py");
    let mut report = AnalysisReport::default();
    for line in 1..=3 {
        report.smells.push(SmellFinding {
            action: ActionLevel::Warning,
            kind: SmellKind::LongFunction,
            message: String::new(),
            file: file.clone(),
            line,
            end_line: line,
            lines: Vec::new(),
            symbol: format!("f{line}"),
            severity: Severity::Warning,
            metric: 1,
            threshold: 1,
            reason: String::new(),
        });
    }
    (report, file)
}
