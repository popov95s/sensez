use super::*;

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
    report.meta.glossary = vec![crate::noze::GlossaryEntry {
        term: "cycles".into(),
        title: "Import Cycle".into(),
        explanation: "modules import each other".into(),
    }];
    assert!(!render(&report, false).contains("What these mean"));
    assert!(render(&report, true).contains("What these mean"));
}

#[test]
fn smell_output_dedupes_matching_action_and_severity() {
    let mut report = AnalysisReport::default();
    report.smells.push(crate::noze::SmellFinding {
        action: ActionLevel::Warning,
        kind: crate::noze::SmellKind::MagicStringDefault,
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

    let text = render(&report, false);
    assert!(text.contains("[warning] x.tsx:12"));
    assert!(!text.contains("[warning] [warning]"));
}
