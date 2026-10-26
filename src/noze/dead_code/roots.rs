//! Where the dead-code walk starts: production entry points and test sources.

use super::RuleSets;
use crate::profiles::registry;
use crate::spine::graph::{CodebaseGraph, ModuleNode};
use petgraph::graph::NodeIndex;
use std::collections::{HashMap, HashSet};

/// How a module enters the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    /// Reached outside the import graph: manifest entries, entry globs, tool configs.
    ProdRoot,
    /// Executed by a test runner.
    TestSource,
    /// Live only through imports.
    Module,
}

/// Role of every internal module.
pub(super) fn roles(
    cg: &CodebaseGraph,
    rule_sets: &RuleSets,
    entry_modules: &HashSet<&str>,
) -> HashMap<NodeIndex, Role> {
    cg.graph
        .node_indices()
        .filter(|&idx| !cg.graph[idx].is_external)
        .filter_map(|idx| {
            let node = &cg.graph[idx];
            let rules = rule_sets.for_language(node.language)?;
            let role = if is_prod_root(node, rules, entry_modules) {
                Role::ProdRoot
            } else if rules.test_source_globs.is_match(&node.file_path) {
                Role::TestSource
            } else {
                Role::Module
            };
            Some((idx, role))
        })
        .collect()
}

fn is_prod_root(node: &ModuleNode, rules: &super::RuleSet, entry_modules: &HashSet<&str>) -> bool {
    let stem = node
        .file_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    registry::dead_code_profile(node.language).is_entry_file_stem(&stem)
        || entry_modules.contains(node.module_name.as_str())
        || rules.entry_globs.is_match(&node.file_path)
        || rules.config_globs.is_match(&node.file_path)
        || rules.manifest_entries.contains(&node.file_path)
}
