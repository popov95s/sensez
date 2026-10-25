use super::dynamic::DynamicFacts;
use crate::profiles::registry;
use crate::spine::graph::CodebaseGraph;
use crate::spine::ir::ImportPhase;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub(super) struct Reach {
    pub(super) distance: usize,
    pub(super) dynamic: bool,
}

/// Importers of each node over runtime graph edges and glob matches; the flag marks dynamic edges.
pub(super) fn reverse_edges(
    graph: &CodebaseGraph,
    dynamic: &DynamicFacts,
    nodes: &HashMap<PathBuf, NodeIndex>,
    root: &Path,
) -> HashMap<NodeIndex, Vec<(NodeIndex, bool)>> {
    let computed: HashSet<(&PathBuf, usize)> = dynamic
        .by_file
        .iter()
        .flat_map(|(path, facts)| facts.imports.iter().map(move |import| (path, import.line)))
        .collect();
    let mut reverse: HashMap<NodeIndex, Vec<(NodeIndex, bool)>> = HashMap::new();
    for edge in graph.graph.edge_references() {
        let import = edge.weight();
        if import.phase == ImportPhase::TypeOnly {
            continue;
        }
        let importer = edge.source();
        let is_computed =
            import.is_inline && computed.contains(&(&graph.graph[importer].file_path, import.line));
        reverse
            .entry(edge.target())
            .or_default()
            .push((importer, is_computed));
    }
    let mut resolvers = registry::import_resolvers();
    for (importer_path, facts) in &dynamic.by_file {
        let Some(&importer) = nodes.get(importer_path) else {
            continue;
        };
        let mut resolve_alias = |specifier: &str| {
            resolvers
                .values_mut()
                .find_map(|resolver| resolver.alias_path(importer_path, specifier))
        };
        for patterns in &facts.patterns {
            let targets = super::patterns::matching_nodes(
                importer_path,
                patterns,
                nodes,
                root,
                &mut resolve_alias,
            );
            for target in targets {
                reverse.entry(target).or_default().push((importer, true));
            }
        }
    }
    reverse
}

/// Breadth-first walk from `seeds` to every test that imports them.
pub(super) fn walk_reverse(
    mut seeds: Vec<(NodeIndex, Reach)>,
    reverse: &HashMap<NodeIndex, Vec<(NodeIndex, bool)>>,
    tests: &HashMap<NodeIndex, PathBuf>,
    selected: &mut HashMap<PathBuf, Reach>,
) {
    seeds.sort_by_key(|(_, reach)| reach.distance);
    let mut visited: HashMap<NodeIndex, usize> = HashMap::new();
    let mut queue = VecDeque::new();
    for (node, reach) in seeds {
        if visited
            .get(&node)
            .is_none_or(|distance| reach.distance < *distance)
        {
            visited.insert(node, reach.distance);
            queue.push_back((node, reach));
        }
    }
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
