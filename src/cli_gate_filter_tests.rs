//! Gate decisions under display filters.

use super::tests::{dead, scan_options};
use super::*;
use crate::report::{ActionLevel, AnalysisReport, Confidence, CycleFinding};

#[test]
fn fail_on_ignores_pillars_filtered_out() {
    let mut report = AnalysisReport::default();
    report.cycles.push(CycleFinding {
        action: ActionLevel::Warning,
        modules: vec!["a".into(), "b".into()],
        hint: None,
        edges: Vec::new(),
    });
    report.dead_code.push(dead("unused", Confidence::High));
    let options = spec::ScanOptions {
        dead_code: true,
        fail_on: Some(FailOnNewLevel::Warning),
        ..scan_options()
    };

    output::select(&mut report, &options);

    assert!(!gate_fails(&report, &options));
}

#[test]
fn fail_on_ignores_low_confidence_dead_code() {
    let mut report = AnalysisReport::default();
    let mut guess = dead("guess", Confidence::Low);
    guess.action = ActionLevel::MustFix;
    report.dead_code.push(guess);
    let options = spec::ScanOptions {
        fail_on: Some(FailOnNewLevel::MustFix),
        ..scan_options()
    };

    output::select(&mut report, &options);

    assert!(!gate_fails(&report, &options));
}

#[test]
fn fail_on_still_sees_findings_beyond_display_cap() {
    let mut report = AnalysisReport::default();
    for index in 0..9 {
        report
            .dead_code
            .push(dead(&format!("advisory{index}"), Confidence::High));
    }
    let mut must_fix = dead("must_fix", Confidence::High);
    must_fix.action = ActionLevel::MustFix;
    report.dead_code.push(must_fix);
    let options = spec::ScanOptions {
        max: Some(1),
        fail_on: Some(FailOnNewLevel::MustFix),
        ..scan_options()
    };

    output::select(&mut report, &options);
    let fails = gate_fails(&report, &options);
    output::shape(&mut report, &options);

    assert!(fails);
    assert_eq!(report.dead_code.len(), 1);
    assert_eq!(report.dead_code[0].symbol, "advisory0");
}
