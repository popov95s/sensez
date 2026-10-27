//! Nesting depth and cognitive weight of syntax nodes.

use crate::profiles::lineage::Lineage;
use crate::profiles::node_names::NodeNames;
use tree_sitter::Node;

/// Kinds that increase block-nesting depth.
pub(super) fn is_nesting(kind: &str) -> bool {
    matches!(
        kind,
        "if_statement"
            | "for_statement"
            | "for_in_statement"
            | "while_statement"
            | "do_statement"
            | "switch_statement"
            | "try_statement"
    )
}

/// `&&` / `||` / `??` short-circuit operators add a decision point.
pub(super) fn is_logical(node: Node, src: &[u8]) -> bool {
    node.field("operator")
        .and_then(|o| o.utf8_text(src).ok())
        .is_some_and(|op| matches!(op, "&&" | "||" | "??"))
}

/// Cyclomatic decision points.
pub(super) fn is_branch(kind: &str, node: Node, src: &[u8]) -> bool {
    match kind {
        "if_statement" | "for_statement" | "for_in_statement" | "while_statement"
        | "do_statement" | "catch_clause" | "switch_case" | "ternary_expression" => true,
        "binary_expression" => is_logical(node, src),
        _ => false,
    }
}

/// Cognitive-complexity increment (Sonar-style): control structures cost
/// `1 + nesting`; logical operators a flat `1`.
pub(super) fn cognitive_weight(
    kind: &str,
    lineage: Lineage,
    src: &[u8],
    depth: usize,
) -> Option<usize> {
    let node = lineage.node();
    match kind {
        "if_statement" if is_else_if(lineage) => Some(1),
        "if_statement" | "for_statement" | "for_in_statement" | "while_statement"
        | "do_statement" | "switch_statement" | "ternary_expression" | "catch_clause" => {
            Some(1 + depth)
        }
        "binary_expression" if is_logical(node, src) => Some(1),
        _ => None,
    }
}

/// `else if`: an `if` that is the `alternative` of another `if`.
pub(super) fn is_else_if(node: Lineage) -> bool {
    node.parent(node.node())
        .filter(|parent| parent.kind_name() == "else_clause")
        .and_then(|clause| node.parent(clause))
        .is_some_and(|grand| grand.kind_name() == "if_statement")
}
