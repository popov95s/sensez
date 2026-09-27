use crate::spine::ir::FunctionUnit;
use std::collections::HashMap;
use tree_sitter::Node;

pub(super) fn forward_target(func: Node<'_>, src: &[u8], params: &[String]) -> Option<String> {
    if func.parent().is_some_and(|parent| parent.kind() == "export_statement") {
        return None;
    }
    let body = func.child_by_field_name("body")?;
    let mut cursor = body.walk();
    let mut statements = body.named_children(&mut cursor).filter(|n| n.kind() != "comment");
    let statement = statements.next()?;
    if statement.kind() != "return_statement" || statements.next().is_some() {
        return None;
    }
    let call = statement.named_child(0)?;
    if call.kind() != "call_expression" {
        return None;
    }
    let callee = call.child_by_field_name("function")?;
    if callee.kind() != "identifier" {
        return None;
    }
    let args = call.child_by_field_name("arguments")?;
    let mut cursor = args.walk();
    let passed: Option<Vec<_>> = args
        .named_children(&mut cursor)
        .map(|arg| (arg.kind() == "identifier").then(|| arg.utf8_text(src).ok()).flatten())
        .collect();
    (params.len() >= 1 && passed? == params.iter().map(String::as_str).collect::<Vec<_>>())
        .then(|| callee.utf8_text(src).ok().map(str::to_string))
        .flatten()
}

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
