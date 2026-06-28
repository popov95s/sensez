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
