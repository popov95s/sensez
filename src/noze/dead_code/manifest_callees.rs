//! Initializer callees that a whole-namespace type manifest shows to be registrations.

use super::demand::Inbound;
use crate::profiles::walk::last_segment;
use crate::spine::graph::CodebaseGraph;
use crate::spine::parser::{ParsedFile, SymbolKind};
use petgraph::graph::NodeIndex;
use std::collections::{HashMap, HashSet};

/// Fewest manifest-exposed exports a callee must initialize to count as a registration.
const MIN_REGISTRATIONS: usize = 5;

/// Callees (`export const x = callee(...)`) repeated across modules a type manifest exposes.
pub(super) struct ManifestCallees {
    names: HashSet<String>,
}

impl ManifestCallees {
    pub(super) fn new(
        cg: &CodebaseGraph,
        files: &[ParsedFile],
        inbound: &HashMap<NodeIndex, Inbound>,
    ) -> Self {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        let exposed = inbound
            .iter()
            .filter(|(_, inbound)| inbound.type_star)
            .filter_map(|(idx, _)| cg.graph[*idx].source_index)
            .filter_map(|index| files.get(index));
        for file in exposed {
            let symbols = &file.walked.symbols;
            let variables = symbols.decorators.iter().filter(|(name, _)| {
                symbols.declared_kinds.get(*name) == Some(&SymbolKind::Variable)
            });
            for (_, callees) in variables {
                for callee in callees {
                    *counts.entry(last_segment(callee)).or_default() += 1;
                }
            }
        }
        let names = counts
            .into_iter()
            .filter(|(_, count)| *count >= MIN_REGISTRATIONS)
            .map(|(name, _)| name.to_string())
            .collect();
        Self { names }
    }

    /// One of `callees` is a manifest registration.
    pub(super) fn registers(&self, callees: Option<&Vec<String>>) -> bool {
        callees.is_some_and(|callees| {
            callees
                .iter()
                .any(|callee| self.names.contains(last_segment(callee)))
        })
    }
}
