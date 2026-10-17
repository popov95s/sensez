use super::module_root::module_root;
use crate::reflexez::patterns::normalize_path;
use crate::spine::graph::CodebaseGraph;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// External graph nodes keyed by the file they name, e.g. `./widget.module.css` or `@/logo.svg?url`.
pub fn asset_nodes(
    graph: &CodebaseGraph,
    configured: &[PathBuf],
) -> HashMap<PathBuf, Vec<NodeIndex>> {
    let mut assets: HashMap<PathBuf, Vec<NodeIndex>> = HashMap::new();
    for index in graph.graph.node_indices() {
        let node = &graph.graph[index];
        if !node.is_external {
            continue;
        }
        let key = node.module_name.split('?').next().unwrap_or_default();
        if Path::new(key).extension().is_none() {
            continue;
        }
        for edge in graph.graph.edges_directed(index, Direction::Incoming) {
            let importer = &graph.graph[edge.source()];
            let root = module_root(&importer.file_path, importer.language, configured);
            let targets = assets.entry(normalize_path(&root.join(key))).or_default();
            if !targets.contains(&index) {
                targets.push(index);
            }
        }
    }
    assets
}
