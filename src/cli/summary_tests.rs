//! `--summary` counts.

use super::tests::{dead, scan_options};
use super::*;
use crate::report::{ActionLevel, AnalysisReport, Confidence, CycleFinding};

#[test]
fn summary_counts_only_selected_pillars() {
    let tmp = tempfile::tempdir().unwrap();
    let mut report = AnalysisReport::default();
    report.cycles.push(CycleFinding {
        action: ActionLevel::Warning,
        modules: vec!["a".into(), "b".into()],
        hint: None,
        edges: Vec::new(),
    });
    report.dead_code.push(dead("one", Confidence::High));
    report.dead_code.push(dead("two", Confidence::High));
    let options = spec::ScanOptions {
        dead_code: true,
        summary: true,
        ..scan_options()
    };

    output::select(&mut report, &options);

    assert_eq!(summary_total(tmp.path(), &report), 2);
}

#[test]
fn summary_with_diff_in_clean_repo_is_empty() {
    let Some(repo) = crate::test_support::GitTestRepo::new("other.py", "from other import y\n")
    else {
        return;
    };
    std::fs::write(&repo.file, "from base import x\n").unwrap();
    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&[
        "-c",
        "user.email=sensez@example.test",
        "-c",
        "user.name=Sensez",
        "commit",
        "-m",
        "cycle",
    ]));
    let full = spec::ScanOptions {
        summary: true,
        ..scan_options()
    };
    let diff = spec::ScanOptions {
        diff: true,
        summary: true,
        ..scan_options()
    };

    let full_report = selected_report(&repo.root, &full).unwrap();
    let diff_report = selected_report(&repo.root, &diff).unwrap();

    assert!(summary_total(&repo.root, &full_report) >= 1);
    assert_eq!(summary_total(&repo.root, &diff_report), 0);
}

fn summary_total(root: &std::path::Path, report: &AnalysisReport) -> u64 {
    let summary = crate::config_summary::render(root, report, None).unwrap();
    serde_json::from_str::<serde_json::Value>(&summary).unwrap()["total_count"]
        .as_u64()
        .unwrap()
}
