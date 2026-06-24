//! Code-health analysis: cycles, dead code, duplication, and smells.

pub mod cycles;
pub mod dead_code;
pub mod duplication;
pub mod glossary;
pub mod smells;

pub use crate::report::*;
pub use crate::spine::parser::SymbolKind;

use crate::bonez;
use crate::config::model::{ActionPolicy, Config};
use crate::spine::graph::CodebaseGraph;
use crate::spine::parser::ParsedFile;
use std::collections::BTreeMap;

/// Run every analyzer pillar, rank findings by impact, and aggregate metadata.
pub fn run(files: &[ParsedFile], graph: &CodebaseGraph, config: &Config) -> AnalysisReport {
    let mut cycles = cycles::detect(graph, &config.smells.exclude);
    for cycle in &mut cycles {
        if cycle.action != ActionLevel::Info {
            cycle.action = config.action.cycles;
        }
    }
    cycles.sort_by_key(|c| (action_rank(c.action), std::cmp::Reverse(c.modules.len())));

    let mut dead_code = dead_code::detect(graph, files, &config.dead_code);
    for finding in &mut dead_code {
        finding.action = config.action.dead_code;
    }
    dead_code.sort_by_key(|f| confidence_rank(f.confidence));

    let mut boundary_audit = bonez::audit(graph, &config.boundaries.forbidden);
    for violation in &mut boundary_audit.violations {
        violation.action = config.action.boundaries;
    }
    let mut duplication = duplication::detect(files, &config.duplication);
    for class in &mut duplication {
        if class.action != ActionLevel::Info {
            class.action = config.action.duplication;
        }
    }

    let mut smells = smells::detect(files, graph, &config.smells);
    apply_smell_actions(&mut smells, &config.action);
    smells.sort_by(|a, b| {
        action_rank(a.action)
            .cmp(&action_rank(b.action))
            .then(severity_rank(a.severity).cmp(&severity_rank(b.severity)))
            .then(b.metric.cmp(&a.metric))
    });

    let (internal_edges, external_edges) = edge_stats(graph);
    let mut report = AnalysisReport {
        meta: ReportMeta {
            mode: ReportMode::Full,
            boundaries_configured: !config.boundaries.forbidden.is_empty(),
            internal_edges,
            external_edges,
            files_skipped: 0,
            analyzed_files: files.len(),
            source_lines: files.iter().map(|f| f.lines as usize).sum(),
            cycles_total: cycles.len(),
            dead_code_total: dead_code.len(),
            duplication_total: duplication.len(),
            boundaries_total: boundary_audit.violations.len(),
            smells_total: smells.len(),
            smell_totals: smell_totals(&smells),
            unmatched_boundary_rules: boundary_audit.unmatched_rules,
            issues: Vec::new(),
            glossary: Vec::new(),
        },
        cycles,
        dead_code,
        boundaries: boundary_audit.violations,
        duplication,
        smells,
    };
    report.meta.glossary = glossary::for_report(&report);
    report
}

fn smell_totals(smells: &[SmellFinding]) -> BTreeMap<String, usize> {
    let mut totals = BTreeMap::new();
    for smell in smells {
        *totals.entry(smell.kind.as_str().to_string()).or_default() += 1;
    }
    totals
}

fn apply_smell_actions(smells: &mut [SmellFinding], policy: &ActionPolicy) {
    for smell in smells {
        let detector_default = ActionLevel::from_severity(smell.severity);
        if smell.action == detector_default {
            smell.action = policy.for_smell(smell.kind, smell.severity);
        }
    }
}

/// Truncate each pillar to the top `max` ranked findings (0 = unlimited).
pub fn limit(report: &mut AnalysisReport, max: usize) {
    if max == 0 {
        return;
    }
    report.cycles.truncate(max);
    report.dead_code.truncate(max);
    report.boundaries.truncate(max);
    report.duplication.truncate(max);
    report.smells.truncate(max);
    report.meta.glossary = glossary::for_report(report);
}

fn severity_rank(s: Severity) -> u8 {
    match s {
        Severity::Critical => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

fn action_rank(level: ActionLevel) -> u8 {
    match level {
        ActionLevel::MustFix => 0,
        ActionLevel::Warning => 1,
        ActionLevel::Advisory => 2,
        ActionLevel::Info => 3,
    }
}

fn confidence_rank(c: Confidence) -> u8 {
    match c {
        Confidence::High => 0,
        Confidence::Medium => 1,
        Confidence::Low => 2,
    }
}

fn edge_stats(graph: &CodebaseGraph) -> (usize, usize) {
    use petgraph::visit::EdgeRef;
    let (mut internal, mut external) = (0, 0);
    for edge in graph.graph.edge_references() {
        if graph.graph[edge.target()].is_external {
            external += 1;
        } else {
            internal += 1;
        }
    }
    (internal, external)
}
