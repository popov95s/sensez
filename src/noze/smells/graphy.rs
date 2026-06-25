//! Graph-metric smells: Shotgun-Surgery Hazard and God Module.
//!
//! Both read the existing module-level dependency graph — no history needed.
//! Shotgun-Surgery *Hazard* is predictive: a module depended on by many distinct
//! modules is one whose change forces edits across a wide, scattered blast radius.

use super::make;
use crate::config::smells::{SmellConfig, Smells};
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::profiles::registry;
use crate::spine::graph::CodebaseGraph;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rayon::prelude::*;

pub fn detect(graph: &CodebaseGraph, cfg: &SmellConfig) -> Vec<SmellFinding> {
    let nodes: Vec<_> = graph.graph.node_indices().collect();
    let internal = nodes
        .iter()
        .filter(|&&i| !graph.graph[i].is_external)
        .count();
    nodes
        .par_iter()
        .filter_map(|&idx| {
            let node = &graph.graph[idx];
            if !is_graph_hotspot_candidate(node) {
                return None;
            }
            // Thresholds are per-language (a node's `language` selects the set).
            let lcfg = cfg.for_language(node.language);
            // Blast radius is judged relative to repo size: in a large codebase
            // the core modules are *expected* to have many dependents, so the
            // configured floor scales up to ~15% of internal modules. Small repos
            // keep the configured absolute threshold.
            let blast_floor = lcfg.shotgun_blast_threshold.max(internal * 15 / 100);
            let blast = afferent(graph, idx, &node.module_name);
            let ce = efferent(graph, idx);
            shotgun(node, blast, blast_floor, lcfg).or_else(|| god_module(node, blast, ce, lcfg))
        })
        .collect()
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
fn afferent(graph: &CodebaseGraph, idx: petgraph::graph::NodeIndex, own: &str) -> usize {
    let mut sources: Vec<&str> = graph
        .graph
        .edges_directed(idx, Direction::Incoming)
        .filter(|e| !e.weight().is_module_decl)
        .map(|e| graph.graph[e.source()].module_name.as_str())
        .filter(|m| *m != own)
        .collect();
    sources.sort_unstable();
    sources.dedup();
    sources.len()
}

/// Efferent coupling: distinct modules depended upon via real coupling edges.
fn efferent(graph: &CodebaseGraph, idx: petgraph::graph::NodeIndex) -> usize {
    let mut targets: Vec<_> = graph
        .graph
        .edges_directed(idx, Direction::Outgoing)
        .filter(|e| !e.weight().is_module_decl)
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
    if cfg.disabled.contains(&SmellKind::ShotgunSurgeryHazard) || blast < floor {
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

/// Minimum fan-in *and* fan-out for a true hub. Excludes composition roots /
/// entrypoints (huge fan-out, ~zero fan-in) that aren't god modules.
const GOD_MODULE_MIN_SIDE: usize = 4;

fn god_module(
    node: &crate::spine::graph::ModuleNode,
    ca: usize,
    ce: usize,
    cfg: &Smells,
) -> Option<SmellFinding> {
    let fan = ca + ce;
    // A god module is a hub: depended upon *and* depending widely. Requiring both
    // sides rules out entrypoints (fan-in ~0) and leaf utilities (fan-out ~0).
    if cfg.disabled.contains(&SmellKind::GodModule)
        || fan < cfg.god_module_fan
        || ca < GOD_MODULE_MIN_SIDE
        || ce < GOD_MODULE_MIN_SIDE
    {
        return None;
    }
    Some(make(
        SmellKind::GodModule,
        format!(
            "fan-in {ca} + fan-out {ce} = {fan} (threshold {})",
            cfg.god_module_fan
        ),
        &node.file_path,
        0,
        &node.module_name,
        Severity::Warning,
        fan as u32,
        cfg.god_module_fan as u32,
    ))
}
