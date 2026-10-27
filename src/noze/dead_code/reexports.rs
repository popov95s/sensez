//! Re-exports that no importer asks the barrel for and nothing else accounts for.

use super::confidence::{grade, Evidence, Reached};
use super::demand::{Demand, Inbound};
use super::RuleSet;
use crate::report::{ActionLevel, DeadCodeFinding};
use crate::spine::graph::CodebaseGraph;
use crate::spine::ir::SymbolKind;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;

/// The barrel being checked and what its importers ask of it.
pub(super) struct Barrel<'a> {
    pub idx: NodeIndex,
    pub inbound: &'a Inbound,
    pub counted: &'a Demand,
    pub reached: Reached,
    pub rules: &'a RuleSet,
}

/// One finding per named re-export of the barrel outside `counted` that `origin_accounts`
/// does not settle at the module defining it.
pub(super) fn unconsumed(
    cg: &CodebaseGraph,
    barrel: &Barrel,
    origin_accounts: impl Fn(NodeIndex, &str) -> bool,
) -> Vec<DeadCodeFinding> {
    let node = &cg.graph[barrel.idx];
    let mut findings = Vec::new();
    for edge in cg.graph.edges_directed(barrel.idx, Direction::Outgoing) {
        let import = edge.weight();
        if !import.is_reexport {
            continue;
        }
        for (origin_name, exported) in import.imported_symbols.iter().zip(&import.bindings) {
            if barrel.counted.contains(exported)
                || barrel.rules.is_entrypoint_name(exported)
                || origin_accounts(edge.target(), origin_name)
            {
                continue;
            }
            let grade = grade(&Evidence {
                inbound: barrel.inbound,
                reached: barrel.reached,
                is_default_export: exported == "default",
                test_only: barrel.inbound.test.contains(exported),
            });
            findings.push(DeadCodeFinding {
                action: ActionLevel::Advisory,
                module: node.module_name.clone(),
                symbol: exported.clone(),
                kind: SymbolKind::ReExport,
                confidence: grade.confidence,
                file: node.file_path.clone(),
                line: import.line,
                reason: grade.reason_text(),
            });
        }
    }
    findings
}

/// `idx` takes `name` from another module: imported or re-exported by name, or through `export *`.
pub(super) fn takes_from_elsewhere(cg: &CodebaseGraph, idx: NodeIndex, name: &str) -> bool {
    cg.graph
        .edges_directed(idx, Direction::Outgoing)
        .map(|edge| edge.weight())
        .any(|import| {
            import.bindings.iter().any(|binding| binding == name)
                || (import.is_reexport
                    && import.bindings.is_empty()
                    && import.imported_symbols.iter().any(|s| s == "*"))
        })
}
