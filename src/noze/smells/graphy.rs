//! Graph-metric smells: Shotgun-Surgery Hazard and God Module.
//!
//! Both read the existing module-level dependency graph — no history needed.
//! Shotgun-Surgery *Hazard* is predictive: a module depended on by many distinct
//! modules is one whose change forces edits across a wide, scattered blast radius.

use super::make;
use crate::config::smells::{SmellConfig, Smells};
use crate::profiles::registry;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::graph::{CodebaseGraph, ModuleNode};
use crate::spine::ir::{ImportPhase, SymbolKind};
use crate::spine::parser::ParsedFile;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rayon::prelude::*;

/// Blast radius is judged relative to repo size: in a large codebase the core
/// modules are *expected* to have many dependents, so the configured floor
/// scales up to this percentage of internal modules. Small repos keep the
/// configured absolute threshold.
const BLAST_FLOOR_REPO_PERCENT: usize = 15;

/// `in_scope` drops skipped files both as candidates and as importers.
pub fn detect(
    graph: &CodebaseGraph,
    files: &[ParsedFile],
    cfg: &SmellConfig,
    in_scope: &(impl Fn(&ModuleNode) -> bool + Sync),
) -> Vec<SmellFinding> {
    let nodes: Vec<_> = graph.graph.node_indices().collect();
    let internal = nodes
        .iter()
        .filter(|&&i| !graph.graph[i].is_external)
        .count();
    nodes
        .par_iter()
        .filter_map(|&idx| {
            let node = &graph.graph[idx];
            if !is_graph_hotspot_candidate(node) || !in_scope(node) {
                return None;
            }
            // Thresholds are per-language (a node's `language` selects the set).
            let lcfg = cfg.for_language(node.language);
            let blast_floor = lcfg
                .shotgun_blast_threshold
                .max(internal * BLAST_FLOOR_REPO_PERCENT / 100);
            let blast = afferent(graph, idx, &node.module_name, in_scope, Edges::Any);
            shotgun(node, blast, blast_floor, lcfg).or_else(|| {
                let size = module_size(files.get(node.source_index?)?);
                let runtime_fan_in =
                    afferent(graph, idx, &node.module_name, in_scope, Edges::Runtime);
                god_module(node, runtime_fan_in, efferent(graph, idx), size, lcfg)
            })
        })
        .collect()
}

/// Which importers count toward fan-in: any (change blast radius) or runtime only (coupling).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Edges {
    Any,
    Runtime,
}

impl Edges {
    fn admits(self, phase: ImportPhase) -> bool {
        self == Edges::Any || phase == ImportPhase::Runtime
    }
}

/// API barrels/re-export surfaces are expected to have high fan-in. They still
/// stay in the graph for cycles and reachability; they just are not actionable
/// graph-smell hotspots.
fn is_graph_hotspot_candidate(node: &crate::spine::graph::ModuleNode) -> bool {
    !node.is_external && !registry::module_profile(node.language).is_package_index(&node.file_path)
}

/// Afferent coupling: distinct dependent modules, counting only real coupling
/// edges (containment — `mod x;`, façade re-exports — is hierarchy, not blast
/// radius). The module itself is excluded; siblings count.
fn afferent(
    graph: &CodebaseGraph,
    idx: petgraph::graph::NodeIndex,
    own: &str,
    in_scope: &impl Fn(&ModuleNode) -> bool,
    edges: Edges,
) -> usize {
    let mut sources: Vec<&str> = graph
        .graph
        .edges_directed(idx, Direction::Incoming)
        .filter(|e| !e.weight().is_module_decl)
        .filter(|e| edges.admits(e.weight().phase))
        .map(|e| &graph.graph[e.source()])
        .filter(|source| in_scope(source))
        .map(|source| source.module_name.as_str())
        .filter(|m| *m != own)
        .collect();
    sources.sort_unstable();
    sources.dedup();
    sources.len()
}

/// Efferent coupling: distinct internal modules depended upon at runtime.
/// Packages and type-only imports don't make a module change with its targets.
fn efferent(graph: &CodebaseGraph, idx: petgraph::graph::NodeIndex) -> usize {
    let mut targets: Vec<_> = graph
        .graph
        .edges_directed(idx, Direction::Outgoing)
        .filter(|e| !e.weight().is_module_decl && e.weight().phase == ImportPhase::Runtime)
        .filter(|e| !graph.graph[e.target()].is_external)
        .map(|e| e.target())
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets.len()
}

fn shotgun(
    node: &crate::spine::graph::ModuleNode,
    blast: usize,
    floor: usize,
    cfg: &Smells,
) -> Option<SmellFinding> {
    if !cfg.is_enabled(SmellKind::ShotgunSurgeryHazard) || blast < floor {
        return None; // disabled for this language, or not a hazard
    }
    let sev = if blast >= floor * 2 {
        Severity::Critical
    } else {
        Severity::Warning
    };
    Some(make(
        SmellKind::ShotgunSurgeryHazard,
        format!("depended on by {blast} distinct modules — a change here scatters edits widely"),
        &node.file_path,
        0,
        &node.module_name,
        sev,
        blast as u32,
        floor as u32,
    ))
}

/// Source lines and top-level functions + classes of one module.
#[derive(Clone, Copy)]
struct ModuleSize {
    lines: usize,
    units: usize,
}

fn module_size(file: &ParsedFile) -> ModuleSize {
    let units = file
        .walked
        .symbols
        .declared_kinds
        .values()
        .filter(|kind| matches!(kind, SymbolKind::Function | SymbolKind::Class))
        .count();
    ModuleSize {
        lines: file.lines as usize,
        units,
    }
}

fn god_module(
    node: &crate::spine::graph::ModuleNode,
    ca: usize,
    ce: usize,
    size: ModuleSize,
    cfg: &Smells,
) -> Option<SmellFinding> {
    let fan = ca + ce;
    // A god module is a sizeable hub: depended upon *and* depending widely.
    // Requiring both sides rules out entrypoints (fan-in ~0) and leaf utilities
    // (fan-out ~0); the size gate rules out small, widely imported leaves.
    let hub = ca >= cfg.god_module_min_fan_in
        && ce >= cfg.god_module_min_fan_out
        && fan >= cfg.god_module_fan;
    let sizeable = size.units >= cfg.god_module_min_units
        || (size.units > 0 && size.lines >= cfg.god_module_min_lines);
    if !cfg.is_enabled(SmellKind::GodModule) || !hub || !sizeable {
        return None;
    }
    Some(make(
        SmellKind::GodModule,
        format!(
            "fan-in {ca} + fan-out {ce} = {fan} (threshold {}) — split it by responsibility so each importer depends on the smaller piece it uses",
            cfg.god_module_fan
        ),
        &node.file_path,
        1,
        &node.module_name,
        Severity::Warning,
        fan as u32,
        cfg.god_module_fan as u32,
    ))
}
