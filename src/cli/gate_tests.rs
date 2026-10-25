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
fn gated_run_with_config_issues_says_so() {
    let report = AnalysisReport {
        meta: ReportMeta {
            issues: vec![crate::report::ScanIssue {
                stage: crate::report::ScanStage::Config,
                file: None,
                message: "sensez.toml:2:1: unknown key".to_string(),
            }],
            ..ReportMeta::default()
        },
        ..AnalysisReport::default()
    };
    let gated = spec::ScanOptions {
        fail_on: Some(FailOnNewLevel::Advisory),
        ..scan_options()
    };

    assert_eq!(gate_config_note(&report, &scan_options()), None);
    let note = gate_config_note(&report, &gated).unwrap();
    assert!(note.contains("1 configuration issue(s)"), "{note}");
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

#[test]
fn fail_on_new_sees_a_finding_in_a_non_ascii_file() {
    let Some(repo) = crate::test_support::GitTestRepo::importing("ümlaut.py", "x") else {
        return;
    };
    std::fs::write(&repo.file, "def live():\n    return 1\n").unwrap();
    assert!(repo.commit_all());
    std::fs::write(
        &repo.file,
        "def live():\n    return 1\n\n\ndef orphan():\n    return 2\n",
    )
    .unwrap();

    assert_eq!(
        run_scan(&repo.root, &fail_on_new()).unwrap(),
        ExitCode::FAILURE
    );
}

#[test]
fn fail_on_new_sees_a_function_whose_only_caller_was_removed() {
    let Some(repo) = crate::test_support::GitTestRepo::importing("calls.py", "x") else {
        return;
    };
    std::fs::write(
        &repo.file,
        "def helper():\n    return 1\n\n\ndef live():\n    return helper()\n",
    )
    .unwrap();
    assert!(repo.commit_all());
    std::fs::write(
        &repo.file,
        "def helper():\n    return 1\n\n\ndef live():\n    return 2\n",
    )
    .unwrap();

    assert_eq!(
        run_scan(&repo.root, &fail_on_new()).unwrap(),
        ExitCode::FAILURE
    );
}

fn fail_on_new() -> spec::ScanOptions {
    spec::ScanOptions {
        fail_on_new: Some(FailOnNewLevel::Advisory),
        ..scan_options()
    }
}
