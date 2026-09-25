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
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) fn full(
    path: &Path,
    threshold: Option<usize>,
    max: usize,
) -> Result<(AnalysisReport, Value, Duration)> {
    let start = Instant::now();
    let (mut report, _module_files) = crate::analyze_path_in_service(path, threshold)
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
    let (mut report, module_files) = crate::analyze_path_in_service(path, threshold)
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
    if let Some(changed) = changed {
        finish_diff(&mut report, &changed, &module_files, max);
    } else {
        crate::noze::limit(&mut report, max);
    }
    Ok((report, snapshot, start.elapsed()))
}

pub(super) fn diff_changed(
    path: &Path,
    threshold: Option<usize>,
    max: usize,
    changed: crate::diff::ChangedLines,
) -> Result<(AnalysisReport, Value, Duration)> {
    let start = Instant::now();
    let (mut report, module_files) = crate::analyze_path_in_service(path, threshold)
        .with_context(|| format!("scanning {}", path.display()))?;
    let snapshot = serde_json::to_value(&report).unwrap_or(Value::Null);
    suppress_scan_issues(&mut report);
    finish_diff(&mut report, &changed, &module_files, max);
    Ok((report, snapshot, start.elapsed()))
}

fn suppress_scan_issues(report: &mut AnalysisReport) {
    report.meta.issues.clear();
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
mod tests {
    use super::*;
    use crate::report::{ActionLevel, Severity, SmellFinding, SmellKind};

    #[test]
    fn diff_keeps_changed_findings_beyond_the_full_report_limit() {
        let file = PathBuf::from("changed.py");
        let mut report = AnalysisReport::default();
        for line in 1..=3 {
            report.smells.push(SmellFinding {
                action: ActionLevel::Warning,
                kind: SmellKind::LongFunction,
                message: String::new(),
                file: file.clone(),
                line,
                end_line: line,
                symbol: format!("f{line}"),
                severity: Severity::Warning,
                metric: 1,
                threshold: 1,
                reason: String::new(),
            });
        }
        let mut changed = crate::diff::ChangedLines::default();
        changed.add(&file, 3, 3);

        finish_diff(&mut report, &changed, &HashMap::new(), 1);

        assert_eq!(report.smells.len(), 1);
        assert_eq!(report.smells[0].line, 3);
    }
}
