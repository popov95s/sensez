use super::tests::scan_options;
use super::*;
use crate::report::{AnalysisReport, ReportMeta};

#[test]
fn diff_outside_git_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let options = spec::ScanOptions {
        diff: true,
        ..scan_options()
    };
    let error = format!("{:#}", run_scan(tmp.path(), &options).unwrap_err());
    assert!(error.contains("refusing to fall back"), "{error}");
    assert!(error.contains("git rev-parse"), "{error}");
}

#[test]
fn diff_from_missing_file_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("missing.diff");
    let options = spec::ScanOptions {
        diff_from: Some(missing.display().to_string()),
        ..scan_options()
    };
    let error = format!("{:#}", run_scan(tmp.path(), &options).unwrap_err());
    assert!(error.contains("refusing to fall back"), "{error}");
    assert!(error.contains("reading diff file"), "{error}");
}

#[test]
fn run_scan_warns_about_invalid_config_and_uses_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("sensez.toml"),
        "[action]\ncycles = \"critical\"\n",
    )
    .unwrap();
    let (report, _) = crate::analyze_path(tmp.path(), None).unwrap();
    assert!(
        report
            .meta
            .issues
            .iter()
            .any(|issue| issue.message.contains("sensez.toml:2:10:")),
        "{:?}",
        report.meta.issues
    );
    assert!(run_scan(tmp.path(), &scan_options()).is_ok());
}

#[test]
fn skipped_analyzer_fails_only_gated_runs() {
    let report = AnalysisReport {
        meta: ReportMeta {
            issues: vec![crate::report::ScanIssue {
                stage: crate::report::ScanStage::Analyze,
                file: None,
                message: "duplication.semantic skipped: offline".to_string(),
            }],
            ..ReportMeta::default()
        },
        ..AnalysisReport::default()
    };
    assert!(ensure_complete_for_gate(&report, &scan_options()).is_ok());

    let gated = spec::ScanOptions {
        fail_on: Some(FailOnNewLevel::MustFix),
        ..scan_options()
    };
    let error = ensure_complete_for_gate(&report, &gated).unwrap_err();
    assert_eq!(
        error.to_string(),
        "incomplete scan: duplication.semantic skipped: offline"
    );
}
