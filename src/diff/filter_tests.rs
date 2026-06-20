use super::*;
use crate::noze::{
    ActionLevel, AnalysisReport, BoundaryViolation, CloneOccurrence, Confidence, CycleFinding,
    DeadCodeFinding, ReportMode, Severity, SmellFinding, SmellKind, SymbolKind,
};
use std::path::Path;

fn dead(file: &str, line: usize) -> DeadCodeFinding {
    DeadCodeFinding {
        action: ActionLevel::Advisory,
        module: "m".into(),
        symbol: "f".into(),
        kind: SymbolKind::Function,
        confidence: Confidence::High,
        file: file.into(),
        line,
        reason: String::new(),
    }
}

fn smell(file: &str, line: usize, end_line: usize) -> SmellFinding {
    SmellFinding {
        action: ActionLevel::Warning,
        kind: SmellKind::LongFunction,
        message: String::new(),
        file: file.into(),
        line,
        end_line,
        symbol: "f".into(),
        severity: Severity::Warning,
        metric: 0,
        threshold: 0,
        reason: String::new(),
    }
}

fn clone_class(occ: &[(&str, usize, usize)]) -> CloneClass {
    CloneClass {
        action: ActionLevel::Advisory,
        token_length: 50,
        occurrences: occ
            .iter()
            .map(|&(f, s, e)| CloneOccurrence {
                file: f.into(),
                start_row: s,
                end_row: e,
            })
            .collect(),
        hint: None,
    }
}

#[test]
fn keeps_touched_findings_and_drops_the_rest() {
    let mut changed = ChangedLines::default();
    changed.add(Path::new("a.py"), 10, 20);

    let mut report = AnalysisReport {
        dead_code: vec![
            dead("a.py", 15),
            dead("a.py", 99),
            dead("b.py", 15),
            dead("a.py", 0),
        ],
        smells: vec![
            smell("a.py", 5, 30),
            smell("a.py", 5, 0),
            smell("b.py", 15, 30),
        ],
        duplication: vec![
            clone_class(&[("a.py", 12, 18), ("b.py", 40, 46)]),
            clone_class(&[("b.py", 1, 5), ("c.py", 1, 5)]),
        ],
        cycles: vec![
            CycleFinding {
                action: ActionLevel::Warning,
                modules: vec!["ma".into(), "mb".into()],
                edges: vec![],
            },
            CycleFinding {
                action: ActionLevel::Warning,
                modules: vec!["mc".into()],
                edges: vec![],
            },
        ],
        boundaries: vec![
            BoundaryViolation {
                action: ActionLevel::MustFix,
                from_module: "ma".into(),
                to_module: "mb".into(),
                file: "a.py".into(),
                line: 12,
                rule: "r".into(),
            },
            BoundaryViolation {
                action: ActionLevel::MustFix,
                from_module: "ma".into(),
                to_module: "mb".into(),
                file: "a.py".into(),
                line: 99,
                rule: "r".into(),
            },
        ],
        ..Default::default()
    };
    let module_files = std::collections::HashMap::from([
        ("ma".to_string(), std::path::PathBuf::from("a.py")),
        ("mb".to_string(), std::path::PathBuf::from("b.py")),
        ("mc".to_string(), std::path::PathBuf::from("c.py")),
    ]);

    apply(&mut report, &changed, &module_files);

    assert_eq!(report.meta.mode, ReportMode::Diff);
    let dead_lines: Vec<usize> = report.dead_code.iter().map(|f| f.line).collect();
    assert_eq!(dead_lines, vec![15], "only the touched def line survives");
    assert!(report
        .dead_code
        .iter()
        .all(|f| f.reason == "added_unreferenced"));

    let smell_spans: Vec<(usize, usize)> =
        report.smells.iter().map(|s| (s.line, s.end_line)).collect();
    assert_eq!(smell_spans, vec![(5, 30)], "body-touch keeps the smell");
    assert!(report
        .smells
        .iter()
        .all(|s| s.reason == "introduced_or_touched"));

    assert_eq!(report.duplication.len(), 1);
    assert_eq!(report.cycles.len(), 1, "cycle with a changed module kept");
    assert_eq!(report.cycles[0].modules[0], "ma");
    let boundary_lines: Vec<usize> = report.boundaries.iter().map(|b| b.line).collect();
    assert_eq!(boundary_lines, vec![12]);
}

#[test]
fn dup_hint_distinguishes_reuse_from_fresh_copies() {
    let mut changed = ChangedLines::default();
    changed.add(Path::new("a.py"), 10, 20);

    let reuse = clone_class(&[("a.py", 12, 18), ("b.py", 40, 46), ("c.py", 1, 5)]);
    let hint = dup_hint(&reuse, &changed);
    assert!(hint.contains("b.py:40"), "points at the pre-existing copy");
    assert!(hint.contains("(+1 more)"), "counts further copies");

    let fresh = clone_class(&[("a.py", 12, 18), ("a.py", 19, 20)]);
    assert_eq!(
        dup_hint(&fresh, &changed),
        "2 copies written in this change"
    );
}

#[test]
fn diff_hint_preserves_detector_context() {
    let mut changed = ChangedLines::default();
    changed.add(Path::new("a.py"), 10, 20);
    let mut class = clone_class(&[("a.py", 12, 18), ("b.py", 40, 46)]);
    class.hint = Some("class property overlap: 4 shared typed properties".into());

    let hint = merged_dup_hint(&class, &changed);
    assert!(hint.contains("class property overlap"));
    assert!(hint.contains("b.py:40"));
}
