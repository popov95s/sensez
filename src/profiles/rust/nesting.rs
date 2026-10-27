//! Rust loop, nesting, branch and cognitive-complexity kinds.

use crate::profiles::walk;
use tree_sitter::Node;

pub(super) fn is_loop(kind: &str) -> bool {
    matches!(
        kind,
        "for_expression" | "while_expression" | "loop_expression"
    )
}

pub(super) fn is_nesting(kind: &str) -> bool {
    matches!(
        kind,
        "if_expression"
            | "match_expression"
            | "for_expression"
            | "while_expression"
            | "loop_expression"
    )
}

pub(super) fn is_branch(kind: &str, node: Node, src: &[u8]) -> bool {
    match kind {
        "if_expression" | "match_expression" | "for_expression" | "while_expression" => true,
        "binary_expression" => node
            .child_by_field_name("operator")
            .and_then(|n| walk::node_text(n, src))
            .is_some_and(|op| matches!(op, "&&" | "||")),
        _ => false,
    }
}

pub(super) fn cognitive_weight(kind: &str, depth: usize) -> Option<usize> {
    match kind {
        "if_expression" | "match_expression" | "for_expression" | "while_expression" => {
            Some(1 + depth)
        }
        _ => None,
    }
}
