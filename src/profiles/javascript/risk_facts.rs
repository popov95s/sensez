use crate::spine::ir::FunctionUnit;
use std::collections::HashMap;
use tree_sitter::Node;

pub(super) fn scan(
    unit: &mut FunctionUnit,
    guards: &mut HashMap<u64, usize>,
    node: Node,
    src: &[u8],
) {
    match node.kind() {
        "catch_clause" => unit.review_risks.broad_handlers += 1,
        "if_statement" => record_guard(unit, guards, node, src),
        "binary_expression" if is_empty_fallback(node) => {
            unit.review_risks.empty_fallbacks += 1;
        }
        _ => {}
    }
}

fn record_guard(
    unit: &mut FunctionUnit,
    guards: &mut HashMap<u64, usize>,
    node: Node<'_>,
    src: &[u8],
) {
    let Some(condition) = node.child_by_field_name("condition") else {
        return;
    };
    crate::profiles::guard_fingerprint::record_repeated_guard(
        unit,
        guards,
        condition,
        ancestor_condition(node),
        node.start_position().row + 1,
        src,
    );
}

fn ancestor_condition(node: Node<'_>) -> Option<Node<'_>> {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if parent.kind() == "if_statement" {
            return parent.child_by_field_name("condition");
        }
        current = parent;
    }
    None
}

fn is_empty_fallback(node: Node<'_>) -> bool {
    let is_fallback_operator = (0..node.child_count())
        .filter_map(|index| node.child(index))
        .any(|child| matches!(child.kind(), "||" | "??"));
    is_fallback_operator
        && node
            .child_by_field_name("right")
            .is_some_and(is_empty_value)
}

fn is_empty_value(node: Node<'_>) -> bool {
    matches!(node.kind(), "null" | "array" | "object")
        && (node.kind() == "null" || node.named_child_count() == 0)
}
