use super::*;

#[test]
fn unknown_lines_are_clamped_to_the_first_line() {
    let range = range(0, 0);
    assert_eq!(range.start.line, 0);
    assert_eq!(range.end.line, 1);
}

#[test]
fn action_levels_map_to_lsp_severity() {
    assert_eq!(severity(ActionLevel::MustFix), 1);
    assert_eq!(severity(ActionLevel::Info), 4);
}

#[test]
fn off_never_publishes_diagnostics() {
    assert!(!is_visible(ActionLevel::MustFix, DisplayLevel::Off));
}

#[test]
fn listed_smell_lines_publish_one_diagnostic_each() {
    let report = AnalysisReport {
        smells: vec![crate::report::SmellFinding {
            action: ActionLevel::Warning,
            kind: crate::report::SmellKind::MessageChain,
            message: "chain".into(),
            file: std::env::temp_dir().join("chain.ts"),
            line: 3,
            end_line: 3,
            lines: vec![3, 9],
            symbol: "f".into(),
            severity: crate::report::Severity::Warning,
            metric: 5,
            threshold: 4,
            reason: String::new(),
        }],
        ..Default::default()
    };

    let spans: Vec<(usize, usize)> = from_report(&report)
        .into_values()
        .flatten()
        .map(|d| (d.range.start.line, d.range.end.line))
        .collect();
    assert_eq!(spans, vec![(2, 3), (8, 9)]);
}
