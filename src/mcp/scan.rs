//! The shared MCP scan + record + project pipeline.
//!
//! Both the `noze_sniff` tool (see [`super::handlers::scan_tool`]) and the
//! `noze_gate` end-of-turn hook (see [`super::gate::gate`]) do the same
//! five things after `analyze_path`:
//!
//! 1. apply triaged `false_positive`s (diff scans only)
//! 2. drop scan-issues from the report (so they never leak to clients)
//! 3. demote noisy detectors using the brainz precision signal
//! 4. cap each pillar to its top-N findings (tools only; `0` skips)
//! 5. snapshot the report for fingerprinting, and record the scan event
//!
//! That whole sequence lives in [`run_and_record`]. The gate adds a
//! one-off `repeats::suppress_repeated` step on the returned report
//! (for the auto-defer UX) but the shared core is unchanged.

use crate::brainz::Origin;
use crate::report::{AnalysisReport, ScanIssue, ScanStage};
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use std::time::Instant;

/// Run a scan end-to-end and return the post-processed [`AnalysisReport`]
/// alongside the [`Value`] snapshot persisted to the brainz store.
///
/// `is_diff` triggers `git diff HEAD` and applies triaged suppressions; the
/// snapshot is always a full (non-diff) view because fingerprinting is
/// against the whole codebase. `max = 0` leaves the report untruncated.
pub(super) fn run_and_record(
    path: &Path,
    threshold: Option<usize>,
    max: usize,
    is_diff: bool,
    origin: Origin,
) -> Result<(AnalysisReport, Value)> {
    let start = Instant::now();
    let (changed, diff_issue) = if is_diff {
        match crate::diff::git::changed_vs_head(path) {
            Ok(changed) => (Some(changed), None),
            Err(err) => (None, Some(format!("{err:#}"))),
        }
    } else {
        (None, None)
    };
    let mut report = crate::analyze_path(path, threshold, changed.as_ref())
        .with_context(|| format!("scanning {}", path.display()))?;
    if let Some(message) = diff_issue {
        report.meta.issues.push(ScanIssue {
            stage: ScanStage::Diff,
            file: None,
            message,
        });
        report.meta.files_skipped = report.meta.issues.len();
    }
    if is_diff {
        crate::brainz::apply_suppressions(path, &mut report);
    }
    super::scan_recording::suppress_scan_issues(&mut report);
    crate::brainz::rank_by_precision(path, &mut report);
    crate::noze::limit(&mut report, max);
    let snapshot = super::scan_recording::snapshot_for_recording(path, threshold, &report, is_diff)
        .context("snapshotting report for fingerprinting")?;
    crate::brainz::record_scan(
        path,
        &snapshot,
        start.elapsed().as_millis() as u64,
        threshold,
        origin,
    );
    Ok((report, snapshot))
}
