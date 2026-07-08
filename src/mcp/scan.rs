//! The shared MCP scan pipeline.
//!
//! Both the `noze_sniff` tool (see [`super::handlers::scan_tool`]) and the
//! `noze_gate` end-of-turn hook (see [`super::gate::gate`]) do the same
//! two things after `analyze_path` (which already applies triaged
//! suppressions and precision ranking):
//!
//! 1. drop scan-issues from the report (so they never leak to clients)
//! 2. cap each pillar to its top-N findings (`max = 0` skips)

use crate::report::{AnalysisReport, ScanIssue, ScanStage};
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

pub(super) fn full(
    path: &Path,
    threshold: Option<usize>,
    max: usize,
) -> Result<(AnalysisReport, Value, Duration)> {
    let start = Instant::now();
    let (mut report, _module_files) = crate::analyze_path(path, threshold)
        .with_context(|| format!("scanning {}", path.display()))?;
    let snapshot = serde_json::to_value(&report).unwrap_or(Value::Null);
    suppress_scan_issues(&mut report);
    crate::noze::limit(&mut report, max);
    Ok((report, snapshot, start.elapsed()))
}

pub(super) fn diff(
    path: &Path,
    threshold: Option<usize>,
    max: usize,
) -> Result<(AnalysisReport, Value, Duration)> {
    let start = Instant::now();
    let (changed, diff_error) = match crate::diff::git::changed_vs_head(path) {
        Ok(changed) => (Some(changed), None),
        Err(err) => (None, Some(format!("{err:#}"))),
    };
    let (mut report, module_files) = crate::analyze_path(path, threshold)
        .with_context(|| format!("scanning {}", path.display()))?;
    if let Some(message) = diff_error {
        report.meta.issues.push(ScanIssue {
            stage: ScanStage::Diff,
            file: None,
            message,
        });
        report.meta.files_skipped = report.meta.issues.len();
    }
    let snapshot = serde_json::to_value(&report).unwrap_or(Value::Null);
    suppress_scan_issues(&mut report);
    crate::noze::limit(&mut report, max);
    if let Some(changed) = changed {
        crate::diff::apply(&mut report, &changed, &module_files);
    }
    Ok((report, snapshot, start.elapsed()))
}

fn suppress_scan_issues(report: &mut AnalysisReport) {
    report.meta.issues.clear();
    report.meta.files_skipped = 0;
}
