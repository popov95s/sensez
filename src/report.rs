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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleFinding {
    pub action: ActionLevel,
    pub modules: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub hint: Option<String>,
    /// One representative closed loop through the cycle (the shortest through
    /// the first module by name), in order; `modules` lists every member.
    pub edges: Vec<CycleEdge>,
}

/// An import that participates in a cycle: `from_module` imports `to_module` at
/// `file:line`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleEdge {
    pub from_module: String,
    pub to_module: String,
    pub file: PathBuf,
    pub line: usize,
}

/// How likely a dead-code candidate is a true positive, most likely first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Confidence {
    /// Reached from a root, and no importer asks for this symbol.
    High,
    /// Unused, but evidence suggests hidden use or the module has no root (see `reason`).
    Medium,
    /// Likely used where sensez cannot see (e.g. unresolved member access); dropped by every consumer.
    Low,
}

/// An unreferenced symbol candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Why it is reported: "test_only", "unreachable_module", or diff-mode provenance ("added_unreferenced").
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub reason: String,
}

/// A forbidden import edge that was found in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryViolation {
    pub action: ActionLevel,
    pub from_module: String,
    pub to_module: String,
    pub file: PathBuf,
    pub line: usize,
    pub rule: String,
}

/// One physical location of a structural clone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneOccurrence {
    pub file: PathBuf,
    pub start_row: usize,
    pub end_row: usize,
}

/// A set of locations sharing an identical structural-token run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CloneClass {
    pub action: ActionLevel,
    pub token_length: usize,
    pub occurrences: Vec<CloneOccurrence>,
    /// Diff-mode guidance for an agent; empty in full scans.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub detector: CloneDetector,
    /// Overlapping classes folded into this one.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub absorbed: Vec<AbsorbedClone>,
}

impl CloneClass {
    /// Keep the occurrences `keep` accepts; whether it is still a clone (two or more left).
    pub fn keep_occurrences(&mut self, keep: impl Fn(&CloneOccurrence) -> bool) -> bool {
        self.occurrences.retain(|occurrence| keep(occurrence));
        self.occurrences.len() >= 2
    }
}

/// Which duplication pass produced a clone class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CloneDetector {
    #[default]
    Structural,
    NearMiss,
    Semantic {
        verdict: SemanticVerdict,
    },
    ClassShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticVerdict {
    EditedCopy,
    SameIntentDifferentCode,
}

/// Evidence from a clone class folded into an overlapping one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsorbedClone {
    pub detector: CloneDetector,
    pub token_length: usize,
    pub copies: usize,
}

/// How serious a smell finding is (drives ordering and rendering), most serious first.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    #[default]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Every offending line when the finding aggregates several hits.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub lines: Vec<usize>,
    pub symbol: String,
    #[serde(skip)]
    pub severity: Severity,
    pub metric: u32,
    pub threshold: u32,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub reason: String,
}

impl SmellFinding {
    /// One `(start, end)` span per offending line, else the finding's own range.
    pub(crate) fn spans(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let own = self
            .lines
            .is_empty()
            .then(|| (self.line, self.end_line.max(self.line)));
        own.into_iter()
            .chain(self.lines.iter().map(|&line| (line, line)))
    }
}

/// Whether a report covers the whole repo or is filtered to a change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportMode {
    #[default]
    Full,
    Diff,
}

/// Which phase of a scan produced a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStage {
    Config,
    Discover,
    Diff,
    Parse,
    Graph,
    Analyze,
}

impl std::fmt::Display for ScanStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanStage::Config => f.write_str("config"),
            ScanStage::Discover => f.write_str("discover"),
            ScanStage::Diff => f.write_str("diff"),
            ScanStage::Parse => f.write_str("parse"),
            ScanStage::Graph => f.write_str("graph"),
            ScanStage::Analyze => f.write_str("analyze"),
        }
    }
}

/// One concrete scan problem that reduced fidelity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanIssue {
    pub stage: ScanStage,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub file: Option<PathBuf>,
    pub message: String,
}

/// Run-level metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "SkipCounts::is_empty", default)]
    pub duplication_skipped: SkipCounts,
    #[serde(skip_serializing_if = "SkipCounts::is_empty", default)]
    pub smells_skipped: SkipCounts,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub smell_totals: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub unmatched_boundary_rules: Vec<String>,
    /// Pillars this run left out on request; their totals are 0 by filter, not by a clean scan.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub filtered_pillars: Vec<Pillar>,
    #[serde(skip_serializing_if = "hide_scan_diagnostics", default)]
    pub issues: Vec<ScanIssue>,
    #[serde(skip)]
    pub glossary: Vec<GlossaryEntry>,
}

/// One plain-English definition for a finding category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryEntry {
    pub term: String,
    pub title: String,
    pub explanation: String,
}

mod analysis;
mod pillar;
mod skip_counts;
pub use analysis::AnalysisReport;
pub use pillar::Pillar;
pub use skip_counts::SkipCounts;
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
