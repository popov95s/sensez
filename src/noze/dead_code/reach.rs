//! Reachability walk from production and test roots over the import graph.

use super::roots::Role;
use crate::spine::graph::CodebaseGraph;
use petgraph::graph::NodeIndex;
use petgraph::Direction;
use std::collections::{HashMap, HashSet};

/// Modules production reaches, and the unreached ones counted with it.
pub(super) struct Reach {
    pub prod: HashSet<NodeIndex>,
    /// Modules no root reaches: unimported ones seed the production walk, import cycles join it.
    pub orphans: HashSet<NodeIndex>,
}

pub(super) fn walk(cg: &CodebaseGraph, roles: &HashMap<NodeIndex, Role>) -> Reach {
    let with_role = |role: Role| {
        roles
            .iter()
            .filter(move |(_, r)| **r == role)
            .map(|(idx, _)| *idx)
    };
    let unimported: HashSet<_> = with_role(Role::Module)
        .filter(|&idx| !has_internal_importer(cg, idx))
        .collect();
    let mut prod = reach_from(
        cg,
        with_role(Role::ProdRoot).chain(unimported.iter().copied()),
    );
    let test = reach_from(cg, with_role(Role::TestSource));
    let islands: Vec<_> = with_role(Role::Module)
        .filter(|idx| !prod.contains(idx) && !test.contains(idx))
        .collect();
    prod.extend(islands.iter().copied());
    Reach {
        prod,
        orphans: unimported.into_iter().chain(islands).collect(),
    }
}

fn has_internal_importer(cg: &CodebaseGraph, idx: NodeIndex) -> bool {
    cg.graph
        .neighbors_directed(idx, Direction::Incoming)
        .any(|source| source != idx && !cg.graph[source].is_external)
}

fn reach_from(
    cg: &CodebaseGraph,
    roots: impl IntoIterator<Item = NodeIndex>,
) -> HashSet<NodeIndex> {
    let mut seen = HashSet::new();
    let mut stack: Vec<_> = roots.into_iter().collect();
    while let Some(idx) = stack.pop() {
        if !seen.insert(idx) {
            continue;
        }
        stack.extend(
            cg.graph
                .neighbors_directed(idx, Direction::Outgoing)
                .filter(|next| !cg.graph[*next].is_external && !seen.contains(next)),
        );
    }
    seen
}
