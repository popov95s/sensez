//! Pillar 3: circular-import detection via Tarjan's SCC algorithm.

use crate::config::Cycles;
use crate::globs::compile_validated;
use crate::report::{ActionLevel, CycleEdge, CycleFinding};
use crate::spine::graph::CodebaseGraph;
use crate::spine::ir::{ImportContext, ImportPhase};
use petgraph::algo::tarjan_scc;
use petgraph::graph::{EdgeIndex, EdgeReference, NodeIndex};
use petgraph::visit::{EdgeFiltered, EdgeRef};
use petgraph::Direction;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};

const RUNTIME: &[ImportPhase] = &[ImportPhase::Runtime];
const TYPE_LEVEL: &[ImportPhase] = &[ImportPhase::Runtime, ImportPhase::TypeOnly];

/// Find strongly connected module components; each finding lists every member and one closed loop.
pub fn detect(cg: &CodebaseGraph, config: &Cycles) -> Vec<CycleFinding> {
    let excluded = compile_validated(&config.exclude);
    let runtime = EdgeFiltered::from_fn(&cg.graph, |edge| is_cycle_edge(edge, RUNTIME));
    let runtime_sccs: Vec<Vec<NodeIndex>> = filter_reportable_sccs(
        cg,
        tarjan_scc(&runtime)
            .into_iter()
            .filter(|scc| scc.len() >= 2),
        &excluded,
    );
    let mut findings = findings_for(cg, &runtime_sccs, ActionLevel::Warning, None, RUNTIME);
    if !config.include_type_only {
        return findings;
    }
    let runtime_keys: HashSet<String> = runtime_sccs.iter().map(|scc| scc_key(cg, scc)).collect();

    let type_level = EdgeFiltered::from_fn(&cg.graph, |edge| is_cycle_edge(edge, TYPE_LEVEL));
    let hint = "type-only cycle: may indicate type coupling and dependent abstractions".to_string();
    findings.extend(
        filter_reportable_sccs(
            cg,
            tarjan_scc(&type_level)
                .into_iter()
                .filter(|scc| scc.len() >= 2),
            &excluded,
        )
        .into_iter()
        .filter(|scc| !runtime_keys.contains(&scc_key(cg, scc)))
        .filter(|scc| has_type_only_internal_edge(cg, scc))
        .map(|scc| finding_for(cg, &scc, ActionLevel::Info, Some(hint.clone()), TYPE_LEVEL)),
    );
    findings
}

fn filter_reportable_sccs(
    cg: &CodebaseGraph,
    sccs: impl Iterator<Item = Vec<NodeIndex>>,
    excluded: &globset::GlobSet,
) -> Vec<Vec<NodeIndex>> {
    sccs.filter(|scc| is_reportable_scc(cg, scc, excluded))
        .collect()
}

fn findings_for(
    cg: &CodebaseGraph,
    sccs: &[Vec<NodeIndex>],
    action: ActionLevel,
    hint: Option<String>,
    phases: &[ImportPhase],
) -> Vec<CycleFinding> {
    sccs.iter()
        .map(|scc| CycleFinding {
            action,
            modules: scc
                .iter()
                .map(|&n| cg.graph[n].module_name.clone())
                .collect(),
            hint: hint.clone(),
            edges: shortest_cycle(cg, scc, phases),
        })
        .collect()
}

fn finding_for(
    cg: &CodebaseGraph,
    scc: &[NodeIndex],
    action: ActionLevel,
    hint: Option<String>,
    phases: &[ImportPhase],
) -> CycleFinding {
    findings_for(cg, &[scc.to_vec()], action, hint, phases)
        .pop()
        .unwrap_or_else(|| CycleFinding {
            action,
            modules: Vec::new(),
            hint: None,
            edges: Vec::new(),
        })
}

fn is_reportable_scc(cg: &CodebaseGraph, scc: &[NodeIndex], excluded: &globset::GlobSet) -> bool {
    scc.iter()
        .any(|&n| !excluded.is_match(&cg.graph[n].file_path))
}

/// Shortest closed import path through the SCC's lexicographically smallest
/// module (BFS), over the same edges the SCC pass used.
fn shortest_cycle(cg: &CodebaseGraph, scc: &[NodeIndex], phases: &[ImportPhase]) -> Vec<CycleEdge> {
    let members: HashSet<NodeIndex> = scc.iter().copied().collect();
    let Some(&start) = scc
        .iter()
        .min_by_key(|&&n| cg.graph[n].module_name.as_str())
    else {
        return Vec::new();
    };
    let mut prev: HashMap<NodeIndex, EdgeIndex> = HashMap::new();
    let mut queue = VecDeque::from([start]);
    while let Some(node) = queue.pop_front() {
        for edge in cg
            .graph
            .edges_directed(node, Direction::Outgoing)
            .filter(|e| members.contains(&e.target()) && is_cycle_edge(*e, phases))
        {
            if edge.target() == start {
                return unwind(cg, &prev, edge.id(), start);
            }
            if let Entry::Vacant(slot) = prev.entry(edge.target()) {
                slot.insert(edge.id());
                queue.push_back(edge.target());
            }
        }
    }
    Vec::new()
}

fn is_cycle_edge(edge: EdgeReference<'_, ImportContext>, phases: &[ImportPhase]) -> bool {
    let import = edge.weight();
    edge.source() != edge.target()
        && !import.is_inline
        && !import.is_module_decl
        && phases.contains(&import.phase)
}

fn unwind(
    cg: &CodebaseGraph,
    prev: &HashMap<NodeIndex, EdgeIndex>,
    closing: EdgeIndex,
    start: NodeIndex,
) -> Vec<CycleEdge> {
    let mut path = vec![closing];
    let mut node = cg.graph.edge_endpoints(closing).map(|(from, _)| from);
    while let Some(current) = node.filter(|&n| n != start) {
        let edge = *prev
            .get(&current)
            .expect("BFS records a predecessor for every reached node");
        path.push(edge);
        node = cg.graph.edge_endpoints(edge).map(|(from, _)| from);
    }
    path.reverse();
    path.into_iter()
        .filter_map(|edge| cycle_edge(cg, edge))
        .collect()
}

fn cycle_edge(cg: &CodebaseGraph, edge: EdgeIndex) -> Option<CycleEdge> {
    let (from, to) = cg.graph.edge_endpoints(edge)?;
    Some(CycleEdge {
        from_module: cg.graph[from].module_name.clone(),
        to_module: cg.graph[to].module_name.clone(),
        file: cg.graph[from].file_path.clone(),
        line: cg.graph[edge].line,
    })
}

fn has_type_only_internal_edge(cg: &CodebaseGraph, scc: &[NodeIndex]) -> bool {
    let members: HashSet<NodeIndex> = scc.iter().copied().collect();
    scc.iter().any(|&node| {
        cg.graph
            .edges_directed(node, Direction::Outgoing)
            .any(|edge| {
                members.contains(&edge.target())
                    && edge.weight().phase == ImportPhase::TypeOnly
                    && is_cycle_edge(edge, TYPE_LEVEL)
            })
    })
}

fn scc_key(cg: &CodebaseGraph, scc: &[NodeIndex]) -> String {
    let mut modules: Vec<&str> = scc
        .iter()
        .map(|&n| cg.graph[n].module_name.as_str())
        .collect();
    modules.sort_unstable();
    modules.join("|")
}

#[cfg(test)]
#[path = "cycles_tests.rs"]
mod tests;
