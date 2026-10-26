use crate::profiles::lineage::Lineage;
use crate::profiles::test_oracle::{self, CallPattern, TestChecks};
use crate::spine::ir::FunctionUnit;
use std::collections::HashMap;
use tree_sitter::Node;

pub(super) fn forward_target(func: Node<'_>, src: &[u8], params: &[String]) -> Option<String> {
    if func
        .parent()
        .is_some_and(|parent| parent.kind() == "export_statement")
    {
        return None;
    }
    let body = func.child_by_field_name("body")?;
    let mut cursor = body.walk();
    let mut statements = body
        .named_children(&mut cursor)
        .filter(|n| n.kind() != "comment");
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
        .map(|arg| {
            (arg.kind() == "identifier")
                .then(|| arg.utf8_text(src).ok())
                .flatten()
        })
        .collect();
    (!params.is_empty() && passed? == params.iter().map(String::as_str).collect::<Vec<_>>())
        .then(|| callee.utf8_text(src).ok().map(str::to_string))
        .flatten()
}
const TEST_CHECKS: TestChecks = TestChecks {
    statement_kinds: &["throw_statement"],
    call_kind: "call_expression",
    callee_field: "function",
    member_kind: Some("member_expression"),
    member_field: Some("property"),
    calls: &[
        CallPattern::Exact("assert"),
        CallPattern::Exact("fail"),
        CallPattern::Prefix("assert."),
        CallPattern::MemberPrefix {
            callee_prefix: "expect(",
            member_prefix: "to",
        },
        CallPattern::MemberExact {
            callee_prefix: "expect(",
            member: "matchSnapshot",
        },
        CallPattern::MemberPrefix {
            callee_prefix: "expect.",
            member_prefix: "to",
        },
        CallPattern::MemberPrefix {
            callee_prefix: "expectTypeOf(",
            member_prefix: "to",
        },
        CallPattern::Exact("assertType"),
        CallPattern::Exact("expect.assertions"),
        CallPattern::Exact("expect.hasAssertions"),
    ],
    framework_calls: &["expect", "expectTypeOf"],
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
    if let Some(helper) = test_oracle::helper_call(node, src, &TEST_CHECKS) {
        unit.review_risks.helper_calls.push(helper.to_string());
    }
    match node.kind() {
        "catch_clause" => unit.review_risks.broad_handlers += 1,
        "if_statement" => record_guard(unit, guards, node, src),
        "binary_expression" if is_empty_fallback(node) => {
            unit.review_risks.empty_fallbacks += 1;
        }
        _ => {}
    }
}

pub(super) fn is_test_callback(func: Lineage, src: &[u8]) -> bool {
    test_callee(func, src).is_some_and(|name| matches!(name, "test" | "it"))
}

pub(super) fn is_test_framework_callback(func: Lineage, src: &[u8]) -> bool {
    test_callee(func, src).is_some_and(|name| {
        matches!(
            name,
            "describe" | "it" | "test" | "beforeEach" | "afterEach" | "beforeAll" | "afterAll"
        )
    })
}

/// Base name of the test-framework call receiving `func` (`it.only`, `test.each(...)` → `it`, `test`).
fn test_callee<'a>(func: Lineage, src: &'a [u8]) -> Option<&'a str> {
    let args = func
        .parent(func.node())
        .filter(|parent| parent.kind() == "arguments")?;
    let call = func
        .parent(args)
        .filter(|parent| parent.kind() == "call_expression")?;
    base_callee(call.child_by_field_name("function")?, src)
}

/// Identifier under `.only` / `.each` / `.concurrent` modifiers and curried calls.
fn base_callee<'a>(callee: Node<'_>, src: &'a [u8]) -> Option<&'a str> {
    match callee.kind() {
        "identifier" => callee.utf8_text(src).ok(),
        "call_expression" => base_callee(callee.child_by_field_name("function")?, src),
        "member_expression" => {
            let modifier = callee
                .child_by_field_name("property")?
                .utf8_text(src)
                .ok()?;
            if !matches!(modifier, "only" | "each" | "concurrent") {
                return None;
            }
            base_callee(callee.child_by_field_name("object")?, src)
        }
        _ => None,
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
