use crate::spine::graph::CodebaseGraph;
use crate::spine::ir::ImportPhase;
use petgraph::graph::NodeIndex;
use petgraph::{visit::EdgeRef, Direction};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectionOfImpact {
    Dependencies,
    Dependents,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImpactOptions {
    pub include_type_only: bool,
}

impl Default for ImpactOptions {
    fn default() -> Self {
        Self {
            include_type_only: true,
        }
    }
}

/// Paths reached from changed files, with shortest import distance.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Reachability {
    pub files: BTreeMap<PathBuf, usize>,
    /// Changed paths missing from the graph; consumers need a safety fallback.
    pub unmapped: BTreeSet<PathBuf>,
}

/// Every internal file transitively reachable from `changed`, at its shortest distance.
pub fn reachable_files(
    graph: &CodebaseGraph,
    changed: &[PathBuf],
    direction: DirectionOfImpact,
    options: ImpactOptions,
) -> Reachability {
    let nodes = internal_nodes_by_path(graph);
    let mut result = Reachability::default();
    let mut queue = VecDeque::new();
    let mut visited = HashMap::new();

    for path in changed {
        let Some(&node) = nodes.get(path) else {
            result.unmapped.insert(path.clone());
            continue;
        };
        if visited.insert(node, 0usize).is_none() {
            queue.push_back(node);
        }
    }

    let edge_direction = match direction {
        DirectionOfImpact::Dependencies => Direction::Outgoing,
        DirectionOfImpact::Dependents => Direction::Incoming,
    };
    while let Some(node) = queue.pop_front() {
        let distance = visited[&node];
        result
            .files
            .insert(graph.graph[node].file_path.clone(), distance);
        for edge in graph.graph.edges_directed(node, edge_direction) {
            if !options.include_type_only && edge.weight().phase == ImportPhase::TypeOnly {
                continue;
            }
            let next = match direction {
                DirectionOfImpact::Dependencies => edge.target(),
                DirectionOfImpact::Dependents => edge.source(),
            };
            if graph.graph[next].is_external || visited.contains_key(&next) {
                continue;
            }
            visited.insert(next, distance + 1);
            queue.push_back(next);
        }
    }
    result
}

fn internal_nodes_by_path(graph: &CodebaseGraph) -> HashMap<PathBuf, NodeIndex> {
    graph
        .graph
        .node_indices()
        .filter_map(|node| {
            let module = &graph.graph[node];
            (!module.is_external).then_some((module.file_path.clone(), node))
        })
        .collect()
}
