//! Parallel scheduling for independent analyzer pillars.

use super::source_scope::PillarScopes;
use super::{cycles, dead_code, duplication, smells};
use crate::bonez::{self, BoundaryAudit};
use crate::config::model::Config;
use crate::report::{CloneClass, CycleFinding, DeadCodeFinding, ScanIssue, SmellFinding};
use crate::spine::graph::CodebaseGraph;
use crate::spine::parser::ParsedFile;
use std::path::Path;

pub(super) struct AnalyzerFindings {
    pub cycles: Vec<CycleFinding>,
    pub dead_code: Vec<DeadCodeFinding>,
    pub boundaries: BoundaryAudit,
    pub duplication: Vec<CloneClass>,
    pub smells: Vec<SmellFinding>,
    pub issues: Vec<ScanIssue>,
}

struct GraphFindings {
    cycles: Vec<CycleFinding>,
    dead_code: Vec<DeadCodeFinding>,
    boundaries: BoundaryAudit,
}

struct SourceFindings {
    duplication: Vec<CloneClass>,
    smells: Vec<SmellFinding>,
    issues: Vec<ScanIssue>,
}

pub(super) fn detect(
    files: &[ParsedFile],
    graph: &CodebaseGraph,
    config: &Config,
    root: Option<&Path>,
    scopes: &PillarScopes,
) -> AnalyzerFindings {
    let (graph_findings, source_findings) = rayon::join(
        || detect_graph_findings(files, graph, config),
        || detect_source_findings(files, graph, config, root, scopes),
    );
    AnalyzerFindings {
        cycles: graph_findings.cycles,
        dead_code: graph_findings.dead_code,
        boundaries: graph_findings.boundaries,
        duplication: source_findings.duplication,
        smells: source_findings.smells,
        issues: source_findings.issues,
    }
}

fn detect_graph_findings(
    files: &[ParsedFile],
    graph: &CodebaseGraph,
    config: &Config,
) -> GraphFindings {
    let (cycles, (dead_code, boundaries)) = rayon::join(
        || cycles::detect(graph, &config.cycles),
        || {
            rayon::join(
                || dead_code::detect(graph, files, &config.dead_code),
                || bonez::audit(graph, &config.boundaries.forbidden),
            )
        },
    );
    GraphFindings {
        cycles,
        dead_code,
        boundaries,
    }
}

fn detect_source_findings(
    files: &[ParsedFile],
    graph: &CodebaseGraph,
    config: &Config,
    root: Option<&Path>,
    scopes: &PillarScopes,
) -> SourceFindings {
    let ((duplication, issues), smells) = rayon::join(
        || duplication::detect_with_root(files, &config.duplication, &scopes.duplication, root),
        || smells::detect(files, graph, &config.smells, &scopes.smells),
    );
    SourceFindings {
        duplication,
        smells,
        issues,
    }
}
