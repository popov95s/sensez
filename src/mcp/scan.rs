//! The shared MCP scan pipeline.

use crate::report::{AnalysisReport, ScanStage};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Which lines of the working tree a scan reports on.
pub(super) enum Scope {
    Full,
    DiffVsHead,
    Changed(crate::diff::ChangedLines),
}

pub(super) fn run(
    path: &Path,
    threshold: Option<usize>,
    max: usize,
    scope: Scope,
) -> Result<(AnalysisReport, Value, Duration)> {
    let start = Instant::now();
    let changed = match scope {
        Scope::Full => None,
        Scope::DiffVsHead => Some(
            crate::diff::git::changed_vs_head(path)
                .with_context(|| format!("resolving working-tree diff for {}", path.display()))?,
        ),
        Scope::Changed(changed) => Some(changed),
    };
    let (mut report, module_files) = crate::analyze_path_in_service(path, threshold)
        .with_context(|| format!("scanning {}", path.display()))?;
    let snapshot = serde_json::to_value(&report).unwrap_or(Value::Null);
    suppress_scan_issues(&mut report);
    match changed {
        Some(changed) => finish_diff(&mut report, &changed, &module_files, max),
        None => {
            report.refresh_totals();
            crate::noze::limit(&mut report, max);
        }
    }
    Ok((report, snapshot, start.elapsed()))
}

fn suppress_scan_issues(report: &mut AnalysisReport) {
    report
        .meta
        .issues
        .retain(|issue| matches!(issue.stage, ScanStage::Analyze | ScanStage::Config));
    report.meta.files_skipped = 0;
}

fn finish_diff(
    report: &mut AnalysisReport,
    changed: &crate::diff::ChangedLines,
    module_files: &HashMap<String, PathBuf>,
    max: usize,
) {
    crate::diff::apply(report, changed, module_files);
    crate::noze::limit(report, max);
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
