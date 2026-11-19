use super::module_node::ModuleNode;
use crate::spine::parser::ImportContext;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// The directed import graph plus a module-name index.
#[derive(Default)]
pub struct CodebaseGraph {
    pub graph: DiGraph<ModuleNode, ImportContext>,
    pub name_to_index: HashMap<String, NodeIndex>,
}
