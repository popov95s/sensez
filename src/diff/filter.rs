//! Per-pillar touch-filter: keep only findings the change is responsible for,
//! attaching provenance an agent can act on.

use super::ChangedLines;
use crate::noze::{
    AnalysisReport, BoundaryViolation, CloneClass, CycleFinding, DeadCodeFinding, SmellFinding,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// Filter `report` in place to diff-relevant findings. `module_files` maps a
/// module name to its file (cycles/boundaries reference modules, not paths).
pub fn apply(
    report: &mut AnalysisReport,
    changed: &ChangedLines,
    module_files: &HashMap<String, PathBuf>,
) {
    report
        .duplication
        .retain(|class| clone_occurrence_touches_diff(class, changed));
    for class in &mut report.duplication {
        class.hint = Some(dup_hint(class, changed));
    }

    report
        .dead_code
        .retain(|finding| dead_symbol_def_touches_diff(finding, changed));
    for finding in &mut report.dead_code {
        finding.reason = "added_unreferenced".to_string();
    }

    report
        .boundaries
        .retain(|violation| boundary_import_touches_diff(violation, changed, module_files));

    report
        .cycles
        .retain(|cycle| cycle_module_touches_diff(cycle, changed, module_files));

    report
        .smells
        .retain(|finding| smell_body_touches_diff(finding, changed));
    for finding in &mut report.smells {
        finding.reason = "introduced_or_touched".to_string();
    }

    report.meta.mode = crate::noze::ReportMode::Diff;
    // Re-scope the glossary to the categories that survived diff filtering.
    report.meta.glossary = crate::noze::glossary::for_report(report);
}

fn clone_occurrence_touches_diff(class: &CloneClass, changed: &ChangedLines) -> bool {
    class.occurrences.iter().any(|occurrence| {
        changed.touches(&occurrence.file, occurrence.start_row, occurrence.end_row)
    })
}

fn dead_symbol_def_touches_diff(finding: &DeadCodeFinding, changed: &ChangedLines) -> bool {
    finding.line > 0 && changed.touches(&finding.file, finding.line, finding.line)
}

fn boundary_import_touches_diff(
    violation: &BoundaryViolation,
    changed: &ChangedLines,
    module_files: &HashMap<String, PathBuf>,
) -> bool {
    module_files
        .get(&violation.from_module)
        .is_some_and(|file| changed.touches(file, violation.line, violation.line))
}

fn cycle_module_touches_diff(
    cycle: &CycleFinding,
    changed: &ChangedLines,
    module_files: &HashMap<String, PathBuf>,
) -> bool {
    cycle.modules.iter().any(|module| {
        module_files
            .get(module)
            .is_some_and(|file| changed.touches_file(file))
    })
}

fn smell_body_touches_diff(finding: &SmellFinding, changed: &ChangedLines) -> bool {
    finding.line > 0
        && changed.touches(
            &finding.file,
            finding.line,
            finding.end_line.max(finding.line),
        )
}

/// n-way-aware guidance: if any copy predates the change, point at it to reuse;
/// otherwise the change itself introduced multiple copies.
fn dup_hint(class: &CloneClass, changed: &ChangedLines) -> String {
    let outside: Vec<_> = class
        .occurrences
        .iter()
        .filter(|o| !changed.touches(&o.file, o.start_row, o.end_row))
        .collect();
    match outside.first() {
        Some(o) => {
            let more = if outside.len() > 1 {
                format!(" (+{} more)", outside.len() - 1)
            } else {
                String::new()
            };
            format!(
                "clone of existing code at {}:{}{} — reuse it if possible, or surface to the user if not",
                o.file.display(),
                o.start_row,
                more
            )
        }
        None => format!("{} copies written in this change", class.occurrences.len()),
    }
}

#[cfg(test)]
mod tests {
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

    /// Each pillar keeps exactly the findings the change is responsible for,
    /// stamps provenance, and flips the report mode.
    #[test]
    fn keeps_touched_findings_and_drops_the_rest() {
        let mut changed = ChangedLines::default();
        changed.add(Path::new("a.py"), 10, 20);

        let mut report = AnalysisReport {
            dead_code: vec![
                dead("a.py", 15), // def line inside the change → kept
                dead("a.py", 99), // untouched line → dropped
                dead("b.py", 15), // untouched file → dropped
                dead("a.py", 0),  // no line info → dropped
            ],
            smells: vec![
                smell("a.py", 5, 30),  // change inside the body → kept
                smell("a.py", 5, 0),   // end unknown, anchor untouched → dropped
                smell("b.py", 15, 30), // untouched file → dropped
            ],
            duplication: vec![
                clone_class(&[("a.py", 12, 18), ("b.py", 40, 46)]), // touched → kept
                clone_class(&[("b.py", 1, 5), ("c.py", 1, 5)]),     // untouched → dropped
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
                    line: 12, // import line inside the change → kept
                    rule: "r".into(),
                },
                BoundaryViolation {
                    action: ActionLevel::MustFix,
                    from_module: "ma".into(),
                    to_module: "mb".into(),
                    file: "a.py".into(),
                    line: 99, // untouched import line → dropped
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

    /// The duplication hint points at a pre-existing copy when one survives
    /// outside the change, and says so when the change wrote every copy.
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
}
