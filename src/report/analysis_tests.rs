use super::*;

#[test]
fn aggregates_actions_across_findings() {
    let report = AnalysisReport {
        duplication: vec![
            CloneClass {
                action: ActionLevel::Warning,
                token_length: 10,
                occurrences: Vec::new(),
                hint: None,
                ..Default::default()
            },
            CloneClass {
                action: ActionLevel::MustFix,
                token_length: 20,
                occurrences: Vec::new(),
                hint: None,
                ..Default::default()
            },
        ],
        ..AnalysisReport::default()
    };

    assert_eq!(report.finding_count(), 2);
    assert_eq!(report.count_at_action(ActionLevel::MustFix), 1);
    assert_eq!(
        report.actions().collect::<Vec<_>>(),
        vec![ActionLevel::Warning, ActionLevel::MustFix]
    );
}

#[test]
fn top_summary_puts_a_must_fix_finding_before_advisory_ones() {
    let orphan = |n: usize| DeadCodeFinding {
        action: ActionLevel::Advisory,
        module: "pkg".into(),
        symbol: format!("orphan{n}"),
        kind: SymbolKind::Function,
        confidence: Confidence::High,
        file: "pkg.py".into(),
        line: n,
        reason: String::new(),
    };
    let report = AnalysisReport {
        dead_code: (1..=6).map(orphan).collect(),
        boundaries: vec![BoundaryViolation {
            action: ActionLevel::MustFix,
            from_module: "core".into(),
            to_module: "api".into(),
            rule: "core-not-api".into(),
            file: "core.py".into(),
            line: 1,
        }],
        ..AnalysisReport::default()
    };

    let summary = report.top_n_summary(5);
    assert!(summary.starts_with("boundary core -> api"), "{summary}");
    assert!(summary.contains("orphan1") && !summary.contains("orphan5"));
}
