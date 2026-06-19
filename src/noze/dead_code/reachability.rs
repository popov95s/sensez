//! Reachability/usage evidence used to decide and rank dead-code candidates.

use crate::noze::Confidence;
use crate::profiles::DeadCodeProfile;
use crate::spine::graph::{CodebaseGraph, ModuleNode};
use globset::GlobSet;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashSet;

/// Inbound import evidence for a module.
pub(super) struct Inbound {
    /// Symbols imported by name across all inbound edges.
    pub used: HashSet<String>,
    /// Any inbound `from mod import *`.
    pub star: bool,
    /// Number of inbound import edges.
    pub count: usize,
}

pub(super) fn inbound_usage(
    cg: &CodebaseGraph,
    idx: NodeIndex,
    skip_source: impl Fn(&ModuleNode) -> bool,
) -> Inbound {
    let mut used = HashSet::new();
    let (mut star, mut count) = (false, 0usize);
    for edge in cg.graph.edges_directed(idx, Direction::Incoming) {
        let source = &cg.graph[edge.source()];
        if skip_source(source) {
            continue;
        }
        count += 1;
        for sym in &edge.weight().imported_symbols {
            if sym == "*" {
                star = true;
            } else {
                used.insert(sym.clone());
            }
        }
    }
    Inbound { used, star, count }
}

/// Cheap, decoration-independent reasons a symbol is not a dead candidate.
pub(super) fn skip_symbol(
    node: &ModuleNode,
    profile: &dyn DeadCodeProfile,
    symbol: &str,
    used: &HashSet<String>,
) -> bool {
    if profile.is_conventionally_private(symbol) {
        return true;
    }
    if node
        .dunder_all
        .as_ref()
        .is_some_and(|all| all.iter().any(|s| s == symbol))
    {
        return true;
    }
    if node.name_counts.get(symbol).copied().unwrap_or(0) > 1 {
        return true; // referenced within its own module
    }
    used.contains(symbol)
}

/// Confidence from what sensez can actually prove (no framework assumptions):
/// - the module is imported *by name* somewhere, yet this symbol never is → **High**;
/// - the module is imported only plainly (`import mod`) → **Medium** (attribute access may hide use);
/// - nothing imports the module → **Low** (it may be an undeclared entry point).
pub(super) fn confidence_of(inbound: &Inbound) -> Confidence {
    if inbound.count == 0 {
        Confidence::Low
    } else if !inbound.used.is_empty() {
        Confidence::High
    } else {
        Confidence::Medium
    }
}

/// Entry-point modules whose symbols are reached outside the import graph.
/// File-stem conventions come from the language profile; package manifests feed
/// `entry_modules`; and profile/user globs feed `entry_points`.
pub(super) fn is_entry_module(
    node: &ModuleNode,
    profile: &dyn DeadCodeProfile,
    entry_modules: &HashSet<&str>,
    entry_globs: &GlobSet,
) -> bool {
    let stem = node
        .file_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if profile.is_entry_file_stem(&stem) {
        return true;
    }
    entry_modules.contains(node.module_name.as_str()) || entry_globs.is_match(&node.file_path)
}
