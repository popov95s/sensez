use anyhow::Result;
use serde_json::Value;
use std::path::Path;

pub(super) fn suppress_scan_issues(report: &mut crate::report::AnalysisReport) {
    report.meta.issues.clear();
    report.meta.files_skipped = 0;
}

pub(super) fn snapshot_for_recording(
    path: &Path,
    threshold: Option<usize>,
    returned_report: &crate::report::AnalysisReport,
    returned_is_diff: bool,
) -> Result<Value> {
    let mut report = if returned_is_diff {
        crate::analyze_path(path, threshold, None)?
    } else {
        returned_report.clone()
    };
    suppress_scan_issues(&mut report);
    Ok(serde_json::to_value(report)?)
}
