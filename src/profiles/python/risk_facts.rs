use crate::profiles::guard_fingerprint::GuardLog;
use crate::profiles::lineage::Lineage;
use crate::profiles::test_oracle::{self, CallPattern, TestChecks};
use crate::profiles::try_handling::{self, TrySyntax};
use crate::spine::ir::FunctionUnit;
use tree_sitter::Node;

const TRY_SYNTAX: TrySyntax = TrySyntax {
    try_kind: "try_statement",
    handler_kind: "except_clause",
    throw_kinds: &["raise_statement"],
    call_kind: "call",
    loggers: &[
        "log*",
        "logging.*",
        "print",
        "*.exception",
        "*.error",
        "*.warning",
    ],
    is_function: super::control_shapes::is_function,
};

pub(super) fn forward_target(func: Node<'_>, src: &[u8], params: &[String]) -> Option<String> {
    if func
        .parent()
        .is_some_and(|parent| parent.kind() == "decorated_definition")
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
    if call.kind() != "call" {
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
    framework_calls: &[],
};

pub(super) fn scan(unit: &mut FunctionUnit, guards: &mut GuardLog, node: Node, src: &[u8]) {
    if test_oracle::is_check(node, src, &TEST_CHECKS) {
        unit.review_risks.test_checks += 1;
    }
    if let Some(helper) = test_oracle::helper_call(node, src, &TEST_CHECKS) {
        unit.review_risks.helper_calls.push(helper.to_string());
    }
    match node.kind() {
        "except_clause" => handler(unit, node, src),
        "if_statement" => guards.record(unit, Lineage::detached(node), &["block"], src),
        "boolean_operator"
            if is_empty_fallback(node) && try_handling::inside_try(node, &TRY_SYNTAX) =>
        {
            unit.review_risks.empty_fallbacks += 1;
        }
        "await" | "yield" => guards.clear(),
        "assignment" | "augmented_assignment" => {
            if let Some(left) = node.child_by_field_name("left") {
                guards.forget_written(left, src);
            }
        }
        _ => {}
    }
}

fn handler(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let bare_body = (node.named_child_count() == 1)
        .then(|| node.named_child(0))
        .flatten()
        .filter(|child| child.kind() == "block");
    if bare_body.is_some_and(|body| try_handling::swallows(body, None, src, &TRY_SYNTAX)) {
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
