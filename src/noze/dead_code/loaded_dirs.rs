//! Directories whose modules a runtime loads by path rather than by import.

use super::roots::Role;
use crate::spine::graph::CodebaseGraph;
use crate::spine::ir::ImportPhase;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashMap;
use std::path::Path;

/// Fewest unimported modules that can make a directory runtime-loaded.
const MIN_UNIMPORTED: usize = 5;

#[derive(Default)]
struct DirCount {
    modules: usize,
    unimported: usize,
}

/// Make modules nothing imports at runtime into production roots when they sit under a
/// directory where at least `share` percent of the modules are like them.
pub(super) fn promote(cg: &CodebaseGraph, roles: &mut HashMap<NodeIndex, Role>, share: u8) {
    if share == 0 {
        return;
    }
    let unimported: Vec<(NodeIndex, bool)> = roles
        .iter()
        .filter(|(_, role)| **role == Role::Module)
        .map(|(&idx, _)| (idx, !has_runtime_importer(cg, roles, idx)))
        .collect();
    let mut dirs: HashMap<&Path, DirCount> = HashMap::new();
    for &(idx, is_unimported) in &unimported {
        for dir in cg.graph[idx].file_path.ancestors().skip(1) {
            let count = dirs.entry(dir).or_default();
            count.modules += 1;
            count.unimported += usize::from(is_unimported);
        }
    }
    let loaded = |path: &Path| {
        path.ancestors().skip(1).any(|dir| {
            dirs.get(dir).is_some_and(|count| {
                count.unimported >= MIN_UNIMPORTED
                    && count.unimported * 100 >= usize::from(share) * count.modules
            })
        })
    };
    let promoted: Vec<NodeIndex> = unimported
        .iter()
        .filter(|(idx, is_unimported)| *is_unimported && loaded(&cg.graph[*idx].file_path))
        .map(|(idx, _)| *idx)
        .collect();
    for idx in promoted {
        roles.insert(idx, Role::ProdRoot);
    }
}

/// Imported by production code in a way that loads it: not by tests, not for types only.
fn has_runtime_importer(
    cg: &CodebaseGraph,
    roles: &HashMap<NodeIndex, Role>,
    idx: NodeIndex,
) -> bool {
    cg.graph
        .edges_directed(idx, Direction::Incoming)
        .any(|edge| {
            let source = edge.source();
            source != idx
                && !cg.graph[source].is_external
                && edge.weight().phase == ImportPhase::Runtime
                && roles.get(&source) != Some(&Role::TestSource)
        })
}
