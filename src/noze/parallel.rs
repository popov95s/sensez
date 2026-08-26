//! Parallel scheduling for independent analyzer pillars.

use super::{cycles, dead_code, duplication, smells};
use crate::bonez::{self, BoundaryAudit};
use crate::config::model::Config;
use crate::report::{CloneClass, CycleFinding, DeadCodeFinding, SmellFinding};
use crate::spine::graph::CodebaseGraph;
use crate::spine::parser::ParsedFile;
use std::path::Path;

pub(super) struct AnalyzerFindings {
    pub cycles: Vec<CycleFinding>,
    pub dead_code: Vec<DeadCodeFinding>,
    pub boundaries: BoundaryAudit,
    pub duplication: Vec<CloneClass>,
    pub smells: Vec<SmellFinding>,
}

struct GraphFindings {
    cycles: Vec<CycleFinding>,
    dead_code: Vec<DeadCodeFinding>,
    boundaries: BoundaryAudit,
}

struct SourceFindings {
    duplication: Vec<CloneClass>,
    smells: Vec<SmellFinding>,
}

pub(super) fn detect(
    files: &[ParsedFile],
    graph: &CodebaseGraph,
    config: &Config,
    root: Option<&Path>,
) -> AnalyzerFindings {
    let (graph_findings, source_findings) = rayon::join(
        || detect_graph_findings(files, graph, config),
        || detect_source_findings(files, graph, config, root),
    );
    AnalyzerFindings {
        cycles: graph_findings.cycles,
        dead_code: graph_findings.dead_code,
        boundaries: graph_findings.boundaries,
        duplication: source_findings.duplication,
        smells: source_findings.smells,
    }
}

fn detect_graph_findings(
    files: &[ParsedFile],
    graph: &CodebaseGraph,
    config: &Config,
) -> GraphFindings {
    let (cycles, (dead_code, boundaries)) = rayon::join(
        || cycles::detect(graph, &config.cycles.exclude),
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
) -> SourceFindings {
    let (duplication, smells) = rayon::join(
        || duplication::detect_with_root(files, &config.duplication, root),
        || smells::detect(files, graph, &config.smells),
    );
    SourceFindings {
        duplication,
        smells,
    }
}
