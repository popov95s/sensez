//! Aggregates parsed files into a directed graph where nodes are modules and
//! edges are import relationships (carrying the full [`ImportContext`]).

mod builder;
mod identity;

pub use builder::build;

use crate::spine::ir::Language;
use crate::spine::parser::ImportContext;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use std::path::PathBuf;

/// A module in the codebase (or a synthetic external dependency).
#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub file_path: PathBuf,
    pub module_name: String,
    /// The language of the source file (drives per-node dead-code dispatch).
    /// External/synthetic nodes inherit the importing module's language.
    pub language: Language,
    /// Index into the immutable parsed-file table; absent for external nodes.
    pub source_index: Option<usize>,
    /// True for stdlib/third-party/unresolved targets outside the scan tree.
    pub is_external: bool,
}

/// The directed import graph plus a module-name index.
#[derive(Default)]
pub struct CodebaseGraph {
    pub graph: DiGraph<ModuleNode, ImportContext>,
    pub name_to_index: HashMap<String, NodeIndex>,
}

#[cfg(test)]
mod tests;
