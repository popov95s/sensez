use crate::profiles::test_oracle::{self, CallPattern, TestChecks};
use crate::spine::ir::FunctionUnit;
use std::collections::HashMap;
use tree_sitter::Node;

const TEST_CHECKS: TestChecks = TestChecks {
    statement_kinds: &["assert_statement", "raise_statement"],
    call_kind: "call",
    callee_field: "function",
    member_kind: None,
    member_field: None,
    calls: &[
        CallPattern::Exact("pytest.raises"),
        CallPattern::Exact("raises"),
        CallPattern::Exact("pytest.warns"),
        CallPattern::Exact("warns"),
        CallPattern::Prefix("self.assert"),
        CallPattern::Prefix("snapshot.assert"),
        CallPattern::Prefix("assert_"),
    ],
};

pub(super) fn scan(
    unit: &mut FunctionUnit,
    guards: &mut HashMap<u64, usize>,
    node: Node,
    src: &[u8],
) {
    if test_oracle::is_check(node, src, &TEST_CHECKS) {
        unit.review_risks.test_checks += 1;
    }
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
