//! Names each module's importers ask for, forwarded through re-export chains.

use super::reach::Reach;
use super::roots::Role;
use crate::spine::graph::CodebaseGraph;
use crate::spine::ir::{ImportContext, ImportPhase};
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

/// Names asked of a module; `all` when an importer takes everything (`from m import *`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Demand {
    pub names: HashSet<String>,
    pub all: bool,
}

impl Demand {
    pub(super) fn contains(&self, name: &str) -> bool {
        self.all || self.names.contains(name)
    }

    /// Add `other`'s names; true when anything was new.
    fn absorb(&mut self, other: &Demand) -> bool {
        let before = (self.all, self.names.len());
        self.all |= other.all;
        self.names.extend(other.names.iter().cloned());
        (self.all, self.names.len()) != before
    }
}

/// Where a module's importers come from and what they ask for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Inbound {
    pub prod: Demand,
    pub test: Demand,
    /// A type-only namespace import used as a whole (`import type * as ns`).
    pub type_star: bool,
    /// An importer that binds the module whole or not at all (`import()`, `import mod`).
    pub opaque: bool,
    /// Imported by a tool config file.
    pub from_config: bool,
}

impl Inbound {
    /// Merge `other` in; true when anything was new.
    fn absorb(&mut self, other: &Inbound) -> bool {
        let flags = (self.type_star, self.opaque, self.from_config);
        let prod = self.prod.absorb(&other.prod);
        let test = self.test.absorb(&other.test);
        self.type_star |= other.type_star;
        self.opaque |= other.opaque;
        self.from_config |= other.from_config;
        prod || test || flags != (self.type_star, self.opaque, self.from_config)
    }

    /// Demand that keeps symbols alive: production importers, plus tests for test-support files.
    pub(super) fn counted(&self, test_support: TestImporters) -> Demand {
        let mut counted = self.prod.clone();
        if test_support == TestImporters::Count {
            counted.absorb(&self.test);
        }
        counted
    }
}

/// Whether test importers keep a module's symbols alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TestImporters {
    Count,
    ReportAsTestOnly,
}

/// Per-importer facts the demand pass needs.
pub(super) struct Importers<'a> {
    pub roles: &'a HashMap<NodeIndex, Role>,
    pub reach: &'a Reach,
    pub config_files: &'a HashSet<NodeIndex>,
}

impl Importers<'_> {
    fn is_test(&self, idx: NodeIndex) -> bool {
        self.roles.get(&idx) == Some(&Role::TestSource) || !self.reach.prod.contains(&idx)
    }

    /// Entry modules and orphans forward every name they re-export.
    fn forwards_everything(&self, idx: NodeIndex) -> bool {
        self.roles.get(&idx) == Some(&Role::ProdRoot) || self.reach.orphans.contains(&idx)
    }
}

/// Inbound demand of every module: direct imports, then re-exports to a fixed point.
pub(super) fn inbound(cg: &CodebaseGraph, importers: &Importers) -> HashMap<NodeIndex, Inbound> {
    let mut demand: HashMap<NodeIndex, Inbound> = HashMap::new();
    let mut reexports = Vec::new();
    for edge in cg.graph.edge_references() {
        if edge.weight().is_reexport {
            reexports.push(edge.id());
            continue;
        }
        let target = demand.entry(edge.target()).or_default();
        add_direct(target, edge.source(), edge.weight(), importers);
    }
    let mut pending = reexports;
    let nothing = Inbound::default();
    while let Some(edge) = pending.pop() {
        let Some((barrel, target)) = cg.graph.edge_endpoints(edge).filter(|(b, t)| b != t) else {
            continue;
        };
        let source = demand.get(&barrel).unwrap_or(&nothing);
        let prod = if importers.forwards_everything(barrel) {
            Cow::Owned(Demand {
                all: true,
                ..source.prod.clone()
            })
        } else {
            Cow::Borrowed(&source.prod)
        };
        let forwarded = forwarded(source, &prod, &cg.graph[edge]);
        if demand.entry(target).or_default().absorb(&forwarded) {
            pending.extend(outgoing_reexports(cg, target));
        }
    }
    demand
}

fn add_direct(
    target: &mut Inbound,
    source: NodeIndex,
    import: &ImportContext,
    importers: &Importers,
) {
    let is_test = importers.is_test(source);
    let mut asked = Demand::default();
    for symbol in &import.imported_symbols {
        match (symbol.as_str(), import.phase) {
            ("*", ImportPhase::TypeOnly) => target.type_star = true,
            ("*", ImportPhase::Runtime) => asked.all = true,
            _ => {
                asked.names.insert(symbol.clone());
            }
        }
    }
    if is_test {
        target.test.absorb(&asked);
        return;
    }
    target.prod.absorb(&asked);
    target.opaque |= is_opaque(import);
    target.from_config |= importers.config_files.contains(&source);
}

/// Binds the module whole or not at all, so which exports it uses is unknown.
fn is_opaque(import: &ImportContext) -> bool {
    let binds_whole = import.imported_symbols.is_empty() || import.bindings.is_empty();
    binds_whole && !import.is_module_decl && !import.imported_symbols.iter().any(|s| s == "*")
}

/// What a re-export edge asks of its target, given the barrel's demand and its production side.
fn forwarded(barrel: &Inbound, prod: &Demand, edge: &ImportContext) -> Inbound {
    Inbound {
        prod: forward_side(prod, edge),
        test: forward_side(&barrel.test, edge),
        type_star: barrel.type_star,
        opaque: barrel.opaque,
        from_config: barrel.from_config,
    }
}

fn forward_side(barrel: &Demand, edge: &ImportContext) -> Demand {
    debug_assert!(
        edge.bindings.is_empty() || edge.bindings.len() == edge.imported_symbols.len(),
        "re-export bindings must align with imported symbols"
    );
    let mut asked = Demand::default();
    for (index, symbol) in edge.imported_symbols.iter().enumerate() {
        match (symbol.as_str(), edge.bindings.get(index)) {
            ("*", Some(namespace)) => asked.all |= barrel.contains(namespace),
            ("*", None) => {
                asked.absorb(barrel);
            }
            (_, exported) => {
                if barrel.contains(exported.unwrap_or(symbol)) {
                    asked.names.insert(symbol.clone());
                }
            }
        }
    }
    asked
}

fn outgoing_reexports(cg: &CodebaseGraph, idx: NodeIndex) -> impl Iterator<Item = EdgeIndex> + '_ {
    cg.graph
        .edges_directed(idx, Direction::Outgoing)
        .filter(|edge| edge.weight().is_reexport)
        .map(|edge| edge.id())
}
