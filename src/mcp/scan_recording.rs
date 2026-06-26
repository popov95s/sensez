use anyhow::Result;
use serde_json::Value;
use std::path::Path;

pub(super) fn suppress_scan_issues(report: &mut crate::report::AnalysisReport) {
    report.meta.issues.clear();
    report.meta.files_skipped = 0;
}

/// Snapshot the codebase for fingerprinting. Always a full re-scan so the
/// fingerprint set is independent of the tool response's `limit` / `max`
/// cap or `diff` filter — those are presentation concerns, not what the
/// metrics should see. A caller asking for `limit: 20` still gets a full
/// `reported_by_detector` count, and a diff scan doesn't pollute the
/// baseline with only the changed files.
pub(super) fn snapshot_for_recording(
    path: &Path,
    threshold: Option<usize>,
    _returned_report: &crate::report::AnalysisReport,
    _returned_is_diff: bool,
) -> Result<Value> {
    let mut report = crate::analyze_path(path, threshold, None)?;
    suppress_scan_issues(&mut report);
    Ok(serde_json::to_value(report)?)
}
