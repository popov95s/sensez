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
        class.hint = Some(merged_dup_hint(class, changed));
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
    refresh_totals(report);
    // Re-scope the glossary to the categories that survived diff filtering.
    report.meta.glossary = crate::noze::glossary::for_report(report);
}

fn refresh_totals(report: &mut AnalysisReport) {
    report.meta.cycles_total = report.cycles.len();
    report.meta.dead_code_total = report.dead_code.len();
    report.meta.boundaries_total = report.boundaries.len();
    report.meta.duplication_total = report.duplication.len();
    report.meta.smells_total = report.smells.len();
    report.meta.smell_totals = smell_totals(&report.smells);
}

fn smell_totals(
    smells: &[crate::report::SmellFinding],
) -> std::collections::BTreeMap<String, usize> {
    let mut totals = std::collections::BTreeMap::new();
    for smell in smells {
        *totals.entry(smell.kind.as_str().to_string()).or_default() += 1;
    }
    totals
}

fn clone_occurrence_touches_diff(class: &CloneClass, changed: &ChangedLines) -> bool {
    class.occurrences.iter().any(|occurrence| {
        changed.touches(&occurrence.file, occurrence.start_row, occurrence.end_row)
    })
}

fn merged_dup_hint(class: &CloneClass, changed: &ChangedLines) -> String {
    let diff_hint = dup_hint(class, changed);
    match class.hint.as_deref().filter(|hint| !hint.is_empty()) {
        Some(detector_hint) => format!("{detector_hint}; {diff_hint}"),
        None => diff_hint,
    }
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
#[path = "filter_tests.rs"]
mod tests;
