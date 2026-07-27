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
        "except_clause" => handler(unit, node, src),
        "if_statement" => record_guard(unit, guards, node, src),
        "boolean_operator" if is_empty_fallback(node) => {
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

fn handler(unit: &mut FunctionUnit, node: Node, _src: &[u8]) {
    let only_body = node.named_child_count() == 1
        && node
            .named_child(0)
            .is_some_and(|child| child.kind() == "block");
    if only_body {
        unit.review_risks.broad_handlers += 1;
    }
}

fn is_empty_fallback(node: Node<'_>) -> bool {
    let is_or = (0..node.child_count())
        .filter_map(|index| node.child(index))
        .any(|child| child.kind() == "or");
    is_or
        && node
            .named_child(node.named_child_count().saturating_sub(1))
            .is_some_and(is_empty_value)
}

fn is_empty_value(node: Node<'_>) -> bool {
    matches!(node.kind(), "none" | "list" | "dictionary")
        && (node.kind() == "none" || node.named_child_count() == 0)
}
