use crate::config::model::{ActionPolicy, Config};
use crate::noze::parallel;
use crate::noze::source_scope::PillarScopes;
use crate::report::{
    ActionLevel, AnalysisReport, Confidence, ReportMeta, ReportMode, SmellFinding,
};
use crate::spine::graph::CodebaseGraph;
use crate::spine::parser::ParsedFile;
use std::path::Path;

#[cfg(test)]
pub fn run(files: &[ParsedFile], graph: &CodebaseGraph, config: &Config) -> AnalysisReport {
    run_with_root(files, graph, config, None)
}

/// Run every analyzer pillar with optional project-root context for analyzers that maintain local caches.
pub fn run_with_root(
    files: &[ParsedFile],
    graph: &CodebaseGraph,
    config: &Config,
    root: Option<&Path>,
) -> AnalysisReport {
    let scopes = PillarScopes::new(config);
    let parallel::AnalyzerFindings {
        mut cycles,
        mut dead_code,
        boundaries: mut boundary_audit,
        mut duplication,
        mut smells,
        issues,
    } = parallel::detect(files, graph, config, root, &scopes);

    for cycle in &mut cycles {
        if cycle.action != ActionLevel::Info {
            cycle.action = config.action.cycles;
        }
    }
    cycles.sort_by_key(|c| (c.action, std::cmp::Reverse(c.modules.len())));

    dead_code.retain(|finding| finding.confidence != Confidence::Low);
    for finding in &mut dead_code {
        finding.action = config.action.dead_code;
    }
    dead_code.sort_by_key(|f| f.confidence);

    for violation in &mut boundary_audit.violations {
        violation.action = config.action.boundaries;
    }
    for class in &mut duplication {
        if class.action != ActionLevel::Info {
            class.action = config.action.duplication;
        }
    }

    apply_smell_actions(&mut smells, &config.action);
    smells.sort_by(|a, b| {
        a.action
            .cmp(&b.action)
            .then(a.severity.cmp(&b.severity))
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
            duplication_skipped: scopes.duplication.skip_counts(files),
            smells_skipped: scopes.smells.skip_counts(files),
            unmatched_boundary_rules: boundary_audit.unmatched_rules,
            issues,
            ..ReportMeta::default()
        },
        cycles,
        dead_code,
        boundaries: boundary_audit.violations,
        duplication,
        smells,
    };
    report.refresh_totals();
    report.meta.glossary = crate::noze::glossary::for_report(&report);
    report
}

fn apply_smell_actions(smells: &mut [SmellFinding], policy: &ActionPolicy) {
    for smell in smells {
        let detector_default = ActionLevel::from_severity(smell.severity);
        if smell.action == detector_default {
            smell.action = policy.for_smell(smell.kind, smell.severity);
        }
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
