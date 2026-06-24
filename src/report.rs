//! The analysis-report data model: one type per finding kind plus run-level
//! metadata and scan diagnostics. Pure data used by analyzers, renderers, diff
//! filtering, and metrics.

use crate::spine::ir::SymbolKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

mod smell_kind;
pub use smell_kind::SmellKind;

/// A circular-import group (Tarjan SCC of cardinality >= 2).
#[derive(Debug, Clone, Serialize)]
pub struct CycleFinding {
    pub action: ActionLevel,
    pub modules: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub hint: Option<String>,
    /// One import edge per module in the cycle (the "next hop"), with the
    /// source file/line so each is clickable.
    pub edges: Vec<CycleEdge>,
}

/// An import that participates in a cycle: `from_module` imports `to_module` at
/// `file:line`.
#[derive(Debug, Clone, Serialize)]
pub struct CycleEdge {
    pub from_module: String,
    pub to_module: String,
    pub file: PathBuf,
    pub line: usize,
}

/// How likely a dead-code candidate is a true positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Confidence {
    /// Nothing in the scan tree imports the module at all.
    High,
    /// The module is imported, but never this symbol by name.
    Medium,
    /// A plain module import may hide use via attribute access.
    Low,
}

/// An unreferenced symbol candidate.
#[derive(Debug, Clone, Serialize)]
pub struct DeadCodeFinding {
    pub action: ActionLevel,
    pub module: String,
    pub symbol: String,
    pub kind: SymbolKind,
    pub confidence: Confidence,
    pub file: PathBuf,
    /// 1-indexed source line; 0 is an internal "unknown/not applicable" sentinel.
    #[serde(skip_serializing_if = "line_is_unknown", default)]
    pub line: usize,
    /// Diff-mode provenance (e.g. "added_unreferenced"); empty otherwise.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub reason: String,
}

/// A forbidden import edge that was found in the graph.
#[derive(Debug, Clone, Serialize)]
pub struct BoundaryViolation {
    pub action: ActionLevel,
    pub from_module: String,
    pub to_module: String,
    pub file: PathBuf,
    pub line: usize,
    pub rule: String,
}

/// One physical location of a structural clone.
#[derive(Debug, Clone, Serialize)]
pub struct CloneOccurrence {
    pub file: PathBuf,
    pub start_row: usize,
    pub end_row: usize,
}

/// A set of locations sharing an identical structural-token run.
#[derive(Debug, Clone, Serialize)]
pub struct CloneClass {
    pub action: ActionLevel,
    pub token_length: usize,
    pub occurrences: Vec<CloneOccurrence>,
    /// Diff-mode guidance for an agent; empty in full scans.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub hint: Option<String>,
}

/// How serious a smell finding is (drives ordering and rendering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

/// How an agent or gate should treat a finding.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ActionLevel {
    MustFix,
    Warning,
    #[default]
    Advisory,
    Info,
}

impl ActionLevel {
    pub const fn advisory() -> Self {
        ActionLevel::Advisory
    }

    pub fn from_severity(severity: Severity) -> Self {
        match severity {
            Severity::Critical => ActionLevel::MustFix,
            Severity::Warning => ActionLevel::Warning,
            Severity::Info => ActionLevel::Advisory,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ActionLevel::MustFix => "must_fix",
            ActionLevel::Warning => "warning",
            ActionLevel::Advisory => "advisory",
            ActionLevel::Info => "info",
        }
    }
}

impl std::fmt::Display for ActionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single design-smell finding.
#[derive(Debug, Clone, Serialize)]
pub struct SmellFinding {
    pub action: ActionLevel,
    pub kind: SmellKind,
    pub message: String,
    pub file: PathBuf,
    /// 1-indexed source line; 0 is an internal "whole module/no anchor" sentinel.
    #[serde(skip_serializing_if = "line_is_unknown", default)]
    pub line: usize,
    #[serde(skip)]
    pub(crate) end_line: usize,
    pub symbol: String,
    pub severity: Severity,
    pub metric: u32,
    pub threshold: u32,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub reason: String,
}

/// Whether a report covers the whole repo or is filtered to a change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportMode {
    #[default]
    Full,
    Diff,
}

/// Which phase of a scan produced a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStage {
    Config,
    Discover,
    Diff,
    Parse,
    Graph,
}

impl std::fmt::Display for ScanStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanStage::Config => f.write_str("config"),
            ScanStage::Discover => f.write_str("discover"),
            ScanStage::Diff => f.write_str("diff"),
            ScanStage::Parse => f.write_str("parse"),
            ScanStage::Graph => f.write_str("graph"),
        }
    }
}

/// One concrete scan problem that reduced fidelity.
#[derive(Debug, Clone, Serialize)]
pub struct ScanIssue {
    pub stage: ScanStage,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub file: Option<PathBuf>,
    pub message: String,
}

/// Run-level metadata.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ReportMeta {
    pub mode: ReportMode,
    pub boundaries_configured: bool,
    pub internal_edges: usize,
    pub external_edges: usize,
    #[serde(skip_serializing_if = "hide_scan_diagnostic_count", default)]
    pub files_skipped: usize,
    pub analyzed_files: usize,
    pub source_lines: usize,
    pub cycles_total: usize,
    pub dead_code_total: usize,
    pub duplication_total: usize,
    pub boundaries_total: usize,
    pub smells_total: usize,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub smell_totals: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub unmatched_boundary_rules: Vec<String>,
    #[serde(skip_serializing_if = "hide_scan_diagnostics", default)]
    pub issues: Vec<ScanIssue>,
    #[serde(skip)]
    pub glossary: Vec<GlossaryEntry>,
}

/// One plain-English definition for a finding category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlossaryEntry {
    pub term: String,
    pub title: String,
    pub explanation: String,
}

/// Aggregate result of all pillars.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AnalysisReport {
    pub meta: ReportMeta,
    pub cycles: Vec<CycleFinding>,
    pub dead_code: Vec<DeadCodeFinding>,
    pub boundaries: Vec<BoundaryViolation>,
    pub duplication: Vec<CloneClass>,
    pub smells: Vec<SmellFinding>,
}

fn line_is_unknown(value: &usize) -> bool {
    *value == 0
}

fn hide_scan_diagnostic_count(value: &usize) -> bool {
    *value == 0 || !scan_diagnostics_enabled()
}

fn hide_scan_diagnostics(value: &[ScanIssue]) -> bool {
    value.is_empty() || !scan_diagnostics_enabled()
}

pub fn scan_diagnostics_enabled() -> bool {
    std::env::var_os("SENSEZ_SCAN_DIAGNOSTICS").is_some()
        || std::env::var_os("SENSEZ_TIMING").is_some()
}
