//! Re-exports that no importer asks the barrel for.

use super::confidence::{grade, Evidence, Reached};
use super::demand::{Demand, Inbound};
use super::RuleSet;
use crate::report::{ActionLevel, DeadCodeFinding};
use crate::spine::graph::CodebaseGraph;
use crate::spine::ir::SymbolKind;
use petgraph::graph::NodeIndex;
use petgraph::Direction;

/// One finding per named re-export of `idx` outside `counted`.
pub(super) fn unconsumed(
    cg: &CodebaseGraph,
    idx: NodeIndex,
    inbound: &Inbound,
    counted: &Demand,
    reached: Reached,
    rules: &RuleSet,
) -> Vec<DeadCodeFinding> {
    let node = &cg.graph[idx];
    cg.graph
        .edges_directed(idx, Direction::Outgoing)
        .map(|edge| edge.weight())
        .filter(|import| import.is_reexport)
        .flat_map(|import| {
            import
                .bindings
                .iter()
                .map(move |exported| (import.line, exported))
        })
        .filter(|(_, exported)| !counted.contains(exported) && !rules.is_entrypoint_name(exported))
        .map(|(line, exported)| {
            let grade = grade(&Evidence {
                inbound,
                reached,
                is_default_export: exported == "default",
                test_only: inbound.test.contains(exported),
            });
            DeadCodeFinding {
                action: ActionLevel::Advisory,
                module: node.module_name.clone(),
                symbol: exported.clone(),
                kind: SymbolKind::ReExport,
                confidence: grade.confidence,
                file: node.file_path.clone(),
                line,
                reason: grade.reason_text(),
            }
        })
        .collect()
}
