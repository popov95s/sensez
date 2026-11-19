//! JSON emitter — the programmatic / CI-CD contract.

use super::ScanDiagnostics;
use crate::report::AnalysisReport;
use anyhow::{Context, Result};

/// Serialize a report to pretty JSON, with or without its scan diagnostics.
pub fn to_json(report: &AnalysisReport, diagnostics: ScanDiagnostics) -> Result<String> {
    let serialized = match diagnostics {
        ScanDiagnostics::Show => serde_json::to_string_pretty(report),
        ScanDiagnostics::Hide => serde_json::to_string_pretty(&without_diagnostics(report)),
    };
    serialized.context("serializing analysis report to JSON")
}

fn without_diagnostics(report: &AnalysisReport) -> AnalysisReport {
    let mut hidden = report.clone();
    hidden.meta.issues.clear();
    hidden.meta.files_skipped = 0;
    hidden
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
