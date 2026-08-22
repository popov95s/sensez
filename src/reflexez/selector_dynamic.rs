use super::dynamic::DynamicFacts;
use crate::spine::graph::CodebaseGraph;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub(super) struct Reach {
    pub(super) distance: usize,
    pub(super) dynamic: bool,
}

pub(super) fn reverse_edges(
    graph: &CodebaseGraph,
    dynamic: &DynamicFacts,
    nodes: &HashMap<PathBuf, NodeIndex>,
    root: &Path,
) -> HashMap<NodeIndex, Vec<(NodeIndex, bool)>> {
    let dynamic_files: HashSet<_> = dynamic
        .by_file
        .iter()
        .filter(|(_, facts)| !facts.imports.is_empty())
        .map(|(path, _)| path)
        .collect();
    let mut reverse: HashMap<NodeIndex, Vec<(NodeIndex, bool)>> = HashMap::new();
    for edge in graph.graph.edge_references() {
        let importer = edge.source();
        if edge.weight().phase == crate::spine::ir::ImportPhase::TypeOnly
            || !dynamic_files.contains(&graph.graph[importer].file_path)
        {
            continue;
        }
        reverse
            .entry(edge.target())
            .or_default()
            .push((importer, true));
    }
    for (importer_path, facts) in &dynamic.by_file {
        let Some(&importer) = nodes.get(importer_path) else {
            continue;
        };
        for pattern in &facts.patterns {
            for target in super::patterns::matching_nodes(importer_path, pattern, nodes, root) {
                reverse.entry(target).or_default().push((importer, true));
            }
        }
    }
    reverse
}

pub(super) fn walk_reverse(
    start: NodeIndex,
    reverse: &HashMap<NodeIndex, Vec<(NodeIndex, bool)>>,
    tests: &HashMap<NodeIndex, PathBuf>,
    selected: &mut HashMap<PathBuf, Reach>,
) {
    let mut queue = VecDeque::from([(
        start,
        Reach {
            distance: 0,
            dynamic: false,
        },
    )]);
    let mut visited = HashMap::from([(start, 0usize)]);
    while let Some((node, reach)) = queue.pop_front() {
        if let Some(path) = tests.get(&node) {
            selected
                .entry(path.clone())
                .and_modify(|current| {
                    if reach.distance < current.distance {
                        *current = reach;
                    } else {
                        current.dynamic |= reach.dynamic;
                    }
                })
                .or_insert(reach);
        }
        for &(importer, dynamic_edge) in reverse.get(&node).into_iter().flatten() {
            let next = Reach {
                distance: reach.distance + 1,
                dynamic: reach.dynamic || dynamic_edge,
            };
            if visited
                .get(&importer)
                .is_some_and(|distance| *distance <= next.distance)
            {
                continue;
            }
            visited.insert(importer, next.distance);
            queue.push_back((importer, next));
        }
    }
}
