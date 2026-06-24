//! CLI-only report shaping: pillar selection plus concise default output.

use super::spec::ScanOptions;
use crate::report::{AnalysisReport, Confidence, SmellFinding};
use std::collections::BTreeMap;

const DEFAULT_TOP: usize = 10;

pub fn apply(report: &mut AnalysisReport, options: &ScanOptions) {
    apply_pillar_filter(report, options);
    if !options.all {
        report
            .dead_code
            .retain(|finding| finding.confidence == Confidence::High);
    }
    refresh_totals(report);
    report.meta.smell_totals = smell_totals(&report.smells);

    if !options.all {
        limit_top(report, options.max.unwrap_or(DEFAULT_TOP));
    } else if let Some(max) = options.max {
        limit_top(report, max);
    }
    report.meta.glossary = crate::noze::glossary::for_report(report);
}

fn apply_pillar_filter(report: &mut AnalysisReport, options: &ScanOptions) {
    let any = options.duplicates
        || options.dead_code
        || options.cycles
        || options.boundaries
        || options.smells;
    if !any {
        return;
    }
    if !options.cycles {
        report.cycles.clear();
    }
    if !options.dead_code {
        report.dead_code.clear();
    }
    if !options.boundaries {
        report.boundaries.clear();
    }
    if !options.duplicates {
        report.duplication.clear();
    }
    if !options.smells {
        report.smells.clear();
    }
}

fn refresh_totals(report: &mut AnalysisReport) {
    report.meta.cycles_total = report.cycles.len();
    report.meta.dead_code_total = report.dead_code.len();
    report.meta.boundaries_total = report.boundaries.len();
    report.meta.duplication_total = report.duplication.len();
    report.meta.smells_total = report.smells.len();
}

fn limit_top(report: &mut AnalysisReport, max: usize) {
    if max == 0 {
        return;
    }
    report.cycles.truncate(max);
    report.dead_code.truncate(max);
    report.boundaries.truncate(max);
    report.duplication.truncate(max);
    limit_smells_by_kind(&mut report.smells, max);
}

fn limit_smells_by_kind(smells: &mut Vec<SmellFinding>, max: usize) {
    let mut seen: BTreeMap<&'static str, usize> = BTreeMap::new();
    smells.retain(|finding| {
        let count = seen.entry(finding.kind.as_str()).or_default();
        let keep = *count < max;
        *count += 1;
        keep
    });
    smells.sort_by(|a, b| {
        a.kind
            .as_str()
            .cmp(b.kind.as_str())
            .then(a.action.cmp(&b.action))
            .then(b.metric.cmp(&a.metric))
    });
}

fn smell_totals(smells: &[SmellFinding]) -> BTreeMap<String, usize> {
    let mut totals = BTreeMap::new();
    for smell in smells {
        *totals.entry(smell.kind.as_str().to_string()).or_default() += 1;
    }
    totals
}
