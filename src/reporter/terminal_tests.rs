use super::*;
use crate::report::{ActionLevel, Severity};

#[test]
fn renders_section_headers() {
    let text = render(&AnalysisReport::default(), false);
    assert!(text.contains("Circular imports"));
    assert!(text.contains("Duplication"));
    assert!(text.contains("Dead code candidates"));
    assert!(text.contains("Boundary violations"));
}

#[test]
fn legend_is_opt_in() {
    let mut report = AnalysisReport::default();
    report.meta.glossary = vec![crate::report::GlossaryEntry {
        term: "cycles".into(),
        title: "Import Cycle".into(),
        explanation: "modules import each other".into(),
    }];
    assert!(!render(&report, false).contains("What these mean"));
    assert!(render(&report, true).contains("What these mean"));
}

#[test]
fn scan_diagnostics_are_hidden_by_default() {
    let mut report = AnalysisReport::default();
    report.meta.files_skipped = 1;
    report.meta.issues.push(crate::report::ScanIssue {
        stage: crate::report::ScanStage::Parse,
        file: Some("broken.py".into()),
        message: "parser detail".into(),
    });

    let text = render(&report, false);

    assert!(!text.contains("scan issue"));
    assert!(!text.contains("parser detail"));
}

#[test]
fn smell_output_shows_only_action_label() {
    let mut report = AnalysisReport::default();
    report.smells.push(crate::report::SmellFinding {
        action: ActionLevel::Warning,
        kind: crate::report::SmellKind::MagicStringDefault,
        message: "sentinel".into(),
        file: "x.tsx".into(),
        line: 12,
        end_line: 12,
        lines: Vec::new(),
        symbol: "Widget".into(),
        severity: Severity::Warning,
        metric: 1,
        threshold: 0,
        reason: String::new(),
    });
    report.smells.push(crate::report::SmellFinding {
        action: ActionLevel::MustFix,
        kind: crate::report::SmellKind::MagicNumbers,
        message: "threshold".into(),
        file: "y.tsx".into(),
        line: 20,
        end_line: 20,
        lines: Vec::new(),
        symbol: "Widget".into(),
        severity: Severity::Info,
        metric: 1,
        threshold: 0,
        reason: String::new(),
    });

    let text = render(&report, false);
    assert!(text.contains("[warning] x.tsx:12"));
    assert!(text.contains("[must_fix] y.tsx:20"));
    assert!(!text.contains("[warning] [warning]"));
    assert!(!text.contains("[must_fix] [info]"));
}

#[test]
fn smell_output_shows_repeated_suggestion_once_per_smell_kind() {
    let mut report = AnalysisReport::default();
    report.smells.push(crate::report::SmellFinding {
        action: ActionLevel::Warning,
        kind: crate::report::SmellKind::LooseTyping,
        message: "params [cfg] — replace loose collections with a typed object or interface".into(),
        file: "a.ts".into(),
        line: 4,
        end_line: 4,
        lines: Vec::new(),
        symbol: "one".into(),
        severity: Severity::Warning,
        metric: 1,
        threshold: 0,
        reason: String::new(),
    });
    report.smells.push(crate::report::SmellFinding {
        action: ActionLevel::Warning,
        kind: crate::report::SmellKind::LooseTyping,
        message: "returns Record<string, any> — replace loose collections with a typed object or interface"
            .into(),
        file: "b.ts".into(),
        line: 8,
        end_line: 8,
        lines: Vec::new(),
        symbol: "two".into(),
        severity: Severity::Warning,
        metric: 1,
        threshold: 0,
        reason: String::new(),
    });

    let text = render(&report, false);
    assert_eq!(
        text.matches("replace loose collections with a typed object or interface")
            .count(),
        1
    );
    assert!(text.contains("params [cfg]"));
    assert!(text.contains("returns Record<string, any>"));
}

#[test]
fn smell_output_names_empty_symbol_anonymous() {
    let mut report = AnalysisReport::default();
    report.smells.push(crate::report::SmellFinding {
        action: ActionLevel::Warning,
        kind: crate::report::SmellKind::MagicNumbers,
        message: "threshold".into(),
        file: "z.ts".into(),
        line: 3,
        end_line: 3,
        symbol: String::new(),
        severity: Severity::Info,
        metric: 1,
        threshold: 0,
        reason: String::new(),
        lines: Vec::new(),
    });

    let text = render(&report, false);
    assert!(text.contains("z.ts:3  <anonymous> — threshold"));
}

#[test]
fn filtered_pillar_is_not_rendered_as_zero() {
    let mut report = AnalysisReport::default();
    report.meta.filtered_pillars = vec![
        crate::report::Pillar::Cycles,
        crate::report::Pillar::Boundaries,
    ];

    let text = render(&report, false);

    assert!(!text.contains("Circular imports"), "{text}");
    assert!(!text.contains("Boundary violations"), "{text}");
    assert!(text.contains("Dead code candidates"), "{text}");
}

#[test]
fn diff_mode_is_announced_in_header() {
    let mut report = AnalysisReport::default();
    assert!(!render(&report, false).contains("diff mode"));
    report.meta.mode = crate::report::ReportMode::Diff;
    assert!(render(&report, false).contains("diff mode: only findings touching changed lines"));
}

#[test]
fn each_pillar_names_the_files_it_skipped() {
    let mut report = AnalysisReport::default();
    report.meta.smells_skipped.tests = 2;
    report.meta.duplication_skipped.generated = 1;

    let text = render(&report, false);

    assert!(
        text.contains(
            "skipped 2 test, 0 generated and 0 excluded file(s) (smells.include_tests / exclude)"
        ),
        "{text}"
    );
    assert!(
        text.contains("skipped 0 test, 1 generated and 0 excluded file(s) (duplication."),
        "{text}"
    );
}
