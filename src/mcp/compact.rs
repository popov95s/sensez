//! Token-conscious MCP report projection.

use crate::report::{
    AnalysisReport, BoundaryViolation, CloneClass, CycleFinding, DeadCodeFinding, ReportMode,
    ScanIssue, SmellFinding,
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct CompactReport {
    meta: CompactMeta,
    cycles: Vec<CycleFinding>,
    dead_code: Vec<DeadCodeFinding>,
    boundaries: Vec<BoundaryViolation>,
    duplication: Vec<CloneClass>,
    smells: Vec<SmellFinding>,
}

#[derive(Serialize)]
struct CompactMeta {
    mode: ReportMode,
    boundaries_configured: bool,
    totals: Totals,
    truncated: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unmatched_boundary_rules: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    issues: Vec<ScanIssue>,
}

/// Findings per pillar before `limit` truncated them.
#[derive(Serialize)]
struct Totals {
    cycles: usize,
    dead_code: usize,
    boundaries: usize,
    duplication: usize,
    smells: usize,
}

pub(super) fn tool_report(report: AnalysisReport) -> CompactReport {
    let totals = Totals {
        cycles: report.meta.cycles_total,
        dead_code: report.meta.dead_code_total,
        boundaries: report.meta.boundaries_total,
        duplication: report.meta.duplication_total,
        smells: report.meta.smells_total,
    };
    let truncated = report.cycles.len() < totals.cycles
        || report.dead_code.len() < totals.dead_code
        || report.boundaries.len() < totals.boundaries
        || report.duplication.len() < totals.duplication
        || report.smells.len() < totals.smells;
    CompactReport {
        meta: CompactMeta {
            mode: report.meta.mode,
            boundaries_configured: report.meta.boundaries_configured,
            totals,
            truncated,
            unmatched_boundary_rules: report.meta.unmatched_boundary_rules,
            issues: report.meta.issues,
        },
        cycles: report.cycles,
        dead_code: report.dead_code,
        boundaries: report.boundaries,
        duplication: report.duplication,
        smells: report.smells,
    }
}
