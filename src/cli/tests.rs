use super::*;
use crate::report::{
    ActionLevel, AnalysisReport, BoundaryViolation, CloneClass, CloneOccurrence, Confidence,
    CycleEdge, CycleFinding, DeadCodeFinding, ReportMeta, ReportMode, Severity, SmellFinding,
    SmellKind,
};
use crate::spine::ir::SymbolKind;
use std::path::PathBuf;

#[test]
fn fail_on_new_blocks_at_configured_level() {
    let mut report = AnalysisReport {
        meta: ReportMeta {
            mode: ReportMode::Diff,
            ..ReportMeta::default()
        },
        ..AnalysisReport::default()
    };
    report.smells.push(SmellFinding {
        action: ActionLevel::Advisory,
        kind: SmellKind::LongFunction,
        message: String::new(),
        file: PathBuf::from("a.py"),
        line: 1,
        end_line: 1,
        symbol: "f".into(),
        severity: Severity::Warning,
        metric: 1,
        threshold: 1,
        reason: String::new(),
    });
    assert!(!report_meets_fail_level(&report, FailOnNewLevel::MustFix));
    assert!(!report_meets_fail_level(&report, FailOnNewLevel::Warning));
    report.smells[0].action = ActionLevel::Warning;
    assert!(report_meets_fail_level(&report, FailOnNewLevel::Warning));
    report.smells[0].action = ActionLevel::MustFix;
    assert!(report_meets_fail_level(&report, FailOnNewLevel::MustFix));

    let _ = (
        CycleFinding {
            action: ActionLevel::Advisory,
            modules: vec![],
            edges: vec![CycleEdge {
                from_module: String::new(),
                to_module: String::new(),
                file: PathBuf::from("a.py"),
                line: 1,
            }],
        },
        DeadCodeFinding {
            action: ActionLevel::Advisory,
            module: String::new(),
            symbol: String::new(),
            kind: SymbolKind::Function,
            confidence: Confidence::High,
            file: PathBuf::from("a.py"),
            line: 1,
            reason: String::new(),
        },
        BoundaryViolation {
            action: ActionLevel::Advisory,
            from_module: String::new(),
            to_module: String::new(),
            file: PathBuf::from("a.py"),
            line: 1,
            rule: String::new(),
        },
        CloneClass {
            action: ActionLevel::Advisory,
            token_length: 1,
            occurrences: vec![CloneOccurrence {
                file: PathBuf::from("a.py"),
                start_row: 1,
                end_row: 1,
            }],
            hint: None,
        },
    );
}
