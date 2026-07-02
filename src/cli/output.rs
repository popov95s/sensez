//! CLI-only report shaping: pillar selection plus concise default output.

use super::spec::ScanOptions;
use crate::report::{AnalysisReport, Confidence, SmellFinding};
use std::collections::BTreeMap;

const DEFAULT_PILLAR_TOP: usize = 5;
const DEFAULT_SMELL_KIND_TOP: usize = 3;

pub fn apply(report: &mut AnalysisReport, options: &ScanOptions) {
    apply_pillar_filter(report, options);
    report
        .dead_code
        .retain(|finding| finding.confidence != Confidence::Low);
    refresh_totals(report);
    report.meta.smell_totals = smell_totals(&report.smells);

    if !options.all {
        limit_top(
            report,
            options.max.unwrap_or(DEFAULT_PILLAR_TOP),
            options.max.unwrap_or(DEFAULT_SMELL_KIND_TOP),
        );
    } else if let Some(max) = options.max {
        limit_top(report, max, max);
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

fn limit_top(report: &mut AnalysisReport, pillar_max: usize, smell_kind_max: usize) {
    if pillar_max == 0 && smell_kind_max == 0 {
        return;
    }
    if pillar_max > 0 {
        report.cycles.truncate(pillar_max);
        report.dead_code.truncate(pillar_max);
        report.boundaries.truncate(pillar_max);
        report.duplication.truncate(pillar_max);
    }
    if smell_kind_max > 0 {
        limit_smells_by_kind(&mut report.smells, smell_kind_max);
    }
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
