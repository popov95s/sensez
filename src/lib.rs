#![doc = include_str!("../README.md")]

mod bonez;
#[cfg(feature = "mcp")]
mod brainz;
mod cli;
pub mod config;
mod config_summary;
mod diff;
mod dotdir;
#[cfg(feature = "eyez")]
mod eyez;
mod fingerprints;
mod globs;
#[cfg(feature = "lsp")]
mod lsp;
#[cfg(feature = "mcp")]
mod mcp;
mod noze;
mod pipeline;
mod profiles;
mod reflexez;
pub mod report;
mod reporter;
mod setup;
mod source_state;
mod spine;
#[cfg(test)]
mod test_support;

pub use cli::run as run_cli;
pub use pipeline::{analyze_path, scan};
pub(crate) use pipeline::{analyze_path_in_service, isolate_service_scans, write_service_scan};
pub use report::{
    AbsorbedClone, ActionLevel, AnalysisReport, BoundaryViolation, CloneClass, CloneDetector,
    CloneOccurrence, Confidence, CycleEdge, CycleFinding, DeadCodeFinding, GlossaryEntry, Pillar,
    ReportMeta, ReportMode, ScanIssue, ScanStage, SemanticVerdict, Severity, SkipCounts,
    SmellFinding, SmellKind,
};
pub use reporter::Format;
