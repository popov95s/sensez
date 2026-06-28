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
    #[serde(skip)]
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

impl AnalysisReport {
    /// Hash over the *content* the gate would nag the agent about
    /// (file + line + kind per finding), not over file mtimes. Two
    /// invocations with the same complaint set get the same hash; an
    /// edit that doesn't touch any of the reported files/lines leaves
    /// the hash alone. Used by the `noze_gate` end-of-turn hook to
    /// avoid re-blocking the same unchanged work when an MCP host
    /// (e.g. anything besides Claude Code's CLI) does not set the
    /// `stop_hook_active` flag.
    ///
    /// Pillar tags prefix each section so a dead-code and a smell on
    /// the same file+line don't collide. Line numbers in a file are
    /// not perturbed by edits to other files, so they're stable across
    /// turns; mtimes would not be.
    pub fn finding_signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = rustc_hash::FxHasher::default();
        "cycles".hash(&mut h);
        for f in &self.cycles {
            for edge in &f.edges {
                edge.file.hash(&mut h);
                edge.line.hash(&mut h);
            }
        }
        "dead_code".hash(&mut h);
        for f in &self.dead_code {
            f.module.hash(&mut h);
            f.symbol.hash(&mut h);
            f.line.hash(&mut h);
        }
        "boundaries".hash(&mut h);
        for f in &self.boundaries {
            f.from_module.hash(&mut h);
            f.to_module.hash(&mut h);
            f.line.hash(&mut h);
        }
        "duplication".hash(&mut h);
        for f in &self.duplication {
            f.token_length.hash(&mut h);
            for occ in &f.occurrences {
                occ.file.hash(&mut h);
                occ.start_row.hash(&mut h);
            }
        }
        "smells".hash(&mut h);
        for f in &self.smells {
            f.file.hash(&mut h);
            f.line.hash(&mut h);
            f.symbol.hash(&mut h);
            f.kind.hash(&mut h);
        }
        h.finish()
    }

    /// One-line per finding up to `max`, joined by `"; "`. The shape is
    /// `pillar/<kind> <symbol-or-module> <file>:<line>` and is what the
    /// `noze_gate` end-of-turn hook relays to the agent. The order is
    /// dead-code → cycles → boundaries → duplication → smells; findings
    /// are not pre-sorted by impact, so the caller is expected to either
    /// cap the report (via `noze::limit`) first or pass a `max` that
    /// matches the pillar order they want.
    pub fn top_n_summary(&self, max: usize) -> String {
        let mut items = Vec::new();
        items.extend(self.dead_code.iter().map(|f| {
            format!(
                "dead_code/{} {}::{} {}:{}",
                f.kind,
                f.module,
                f.symbol,
                f.file.display(),
                f.line
            )
        }));
        items.extend(
            self.cycles
                .iter()
                .map(|f| format!("cycle {}", f.modules.join(" -> "))),
        );
        items.extend(self.boundaries.iter().map(|f| {
            format!(
                "boundary {} -> {} {}:{}",
                f.from_module,
                f.to_module,
                f.file.display(),
                f.line
            )
        }));
        items.extend(self.duplication.iter().map(|f| {
            let locations = f
                .occurrences
                .iter()
                .map(|o| format!("{}:{}", o.file.display(), o.start_row))
                .collect::<Vec<_>>()
                .join(", ");
            format!("duplication {} token(s) at {locations}", f.token_length)
        }));
        items.extend(self.smells.iter().map(|f| {
            format!(
                "smell/{} {} {}:{}",
                f.kind,
                f.symbol,
                f.file.display(),
                f.line
            )
        }));
        items.truncate(max);
        items.join("; ")
    }
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
