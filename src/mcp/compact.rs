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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unmatched_boundary_rules: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    issues: Vec<ScanIssue>,
}

pub(super) fn tool_report(report: AnalysisReport) -> CompactReport {
    CompactReport {
        meta: CompactMeta {
            mode: report.meta.mode,
            boundaries_configured: report.meta.boundaries_configured,
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
