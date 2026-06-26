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
    let mut report = crate::analyze_path(path, threshold, None)
        .with_context(|| format!("scanning {}", path.display()))?;
    finish(path, threshold, max, &mut report, start)
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
    let mut report = crate::analyze_path(path, threshold, changed.as_ref())
        .with_context(|| format!("scanning {}", path.display()))?;
    if let Some(message) = diff_error {
        report.meta.issues.push(ScanIssue {
            stage: ScanStage::Diff,
            file: None,
            message,
        });
        report.meta.files_skipped = report.meta.issues.len();
    }
    finish(path, threshold, max, &mut report, start)
}

/// Shared tail: strip scan issues, cap to top-N, snapshot.
fn finish(
    path: &Path,
    threshold: Option<usize>,
    max: usize,
    report: &mut AnalysisReport,
    start: Instant,
) -> Result<(AnalysisReport, Value, Duration)> {
    super::scan_recording::suppress_scan_issues(report);
    crate::noze::limit(report, max);
    let snapshot = super::scan_recording::snapshot_for_recording(path, threshold, report, false)
        .context("snapshotting report for fingerprinting")?;
    Ok((report.clone(), snapshot, start.elapsed()))
}
