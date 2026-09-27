use crate::spine::ir::FunctionUnit;
use std::collections::HashMap;
use tree_sitter::Node;
use std::path::Path;

pub(crate) fn is_test_source(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.ends_with(".test.js")
                || name.ends_with(".test.ts")
                || name.ends_with(".test.jsx")
                || name.ends_with(".test.tsx")
                || name.ends_with(".spec.js")
                || name.ends_with(".spec.ts")
                || name.ends_with(".spec.jsx")
                || name.ends_with(".spec.tsx")
        })
}

pub(super) fn scan(
    unit: &mut FunctionUnit,
    guards: &mut HashMap<u64, usize>,
    node: Node,
    src: &[u8],
) {
    match node.kind() {
        "throw_statement" => unit.review_risks.test_checks += 1,
        "call_expression" if is_test_check(node, src) => unit.review_risks.test_checks += 1,
        "catch_clause" => unit.review_risks.broad_handlers += 1,
        "if_statement" => record_guard(unit, guards, node, src),
        "binary_expression" if is_empty_fallback(node) => {
            unit.review_risks.empty_fallbacks += 1;
        }
        _ => {}
    }
}

pub(super) fn is_test_callback(func: Node<'_>, src: &[u8]) -> bool {
    let Some(args) = func.parent().filter(|parent| parent.kind() == "arguments") else {
        return false;
    };
    let Some(call) = args
        .parent()
        .filter(|parent| parent.kind() == "call_expression")
    else {
        return false;
    };
    call.child_by_field_name("function")
        .and_then(|callee| callee.utf8_text(src).ok())
        .is_some_and(|name| matches!(name, "test" | "it"))
}

fn is_test_check(node: Node<'_>, src: &[u8]) -> bool {
    let Some(callee) = node.child_by_field_name("function") else {
        return false;
    };
    let Ok(name) = callee.utf8_text(src) else {
        return false;
    };
    if matches!(name, "assert" | "fail") || name.starts_with("assert.") {
        return true;
    }
    if callee.kind() != "member_expression" {
        return false;
    }
    let Some(property) = callee
        .child_by_field_name("property")
        .and_then(|p| p.utf8_text(src).ok())
    else {
        return false;
    };
    (property.starts_with("to") || matches!(property, "matchSnapshot"))
        && name.starts_with("expect(")
        || (property.starts_with("to") && name.starts_with("expect."))
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
