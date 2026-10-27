use crate::profiles::guard_fingerprint::GuardLog;
use crate::profiles::handler_end;
use crate::profiles::lineage::Lineage;
use crate::profiles::node_names::NodeNames;
use crate::profiles::test_oracle::{self, CallPattern, TestChecks};
use crate::profiles::try_handling::{self, TrySyntax};
use crate::spine::ir::{FunctionUnit, HandlerEnd, SentinelHandler};
use tree_sitter::Node;

const TRY_SYNTAX: TrySyntax = TrySyntax {
    try_kind: "try_statement",
    handler_kind: "catch_clause",
    throw_kinds: &["throw_statement"],
    call_kind: "call_expression",
    loggers: &[
        "console.*",
        "log*",
        "*.log",
        "*.error",
        "*.warn",
        "*.info",
        "*.debug",
    ],
    notifiers: &[
        "*toast*",
        "*Toast*",
        "notify*",
        "*.notify*",
        "alert",
        "set*Error*",
    ],
    is_function: super::units::is_function,
    return_kind: "return_statement",
    noop_kinds: &["empty_statement"],
    comment_kind: "comment",
    cleanup_kinds: &["finally_clause"],
    parse_calls: &["JSON.parse"],
    probes: &[
        "*localStorage.*",
        "*sessionStorage.*",
        "stat",
        "access",
        "*.stat",
        "*.access",
    ],
    returned_literal,
};

/// Nodes a repeated guard is scoped to: its block or loop.
const GUARD_SCOPES: &[&str] = &[
    "statement_block",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "do_statement",
];

pub(super) fn forward_target(func: Node<'_>, src: &[u8], params: &[String]) -> Option<String> {
    if func
        .parent()
        .is_some_and(|parent| parent.kind_name() == "export_statement")
    {
        return None;
    }
    let body = func.field("body")?;
    let mut cursor = body.walk();
    let mut statements = body
        .named_children(&mut cursor)
        .filter(|n| n.kind_name() != "comment");
    let statement = statements.next()?;
    if statement.kind_name() != "return_statement" || statements.next().is_some() {
        return None;
    }
    let call = statement.named_child(0)?;
    if call.kind_name() != "call_expression" {
        return None;
    }
    let callee = call.field("function")?;
    if callee.kind_name() != "identifier" {
        return None;
    }
    let args = call.field("arguments")?;
    let mut cursor = args.walk();
    let passed: Option<Vec<_>> = args
        .named_children(&mut cursor)
        .map(|arg| {
            (arg.kind_name() == "identifier")
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
    guards: &mut GuardLog,
    lineage: Lineage,
    src: &[u8],
    frame: Option<usize>,
) {
    let node = lineage.node();
    super::echo_facts::scan(unit, node, src);
    if test_oracle::is_check(node, src, &TEST_CHECKS) {
        unit.review_risks.test_checks += 1;
    }
    if let Some(helper) = test_oracle::helper_call(node, src, &TEST_CHECKS) {
        unit.review_risks.helper_calls.push(helper.to_string());
    }
    match node.kind_name() {
        "catch_clause" => {
            if swallows(node, src) {
                unit.review_risks.broad_handlers += 1;
            }
            unit.review_risks
                .sentinel_handlers
                .extend(sentinel_handler(node, src, frame));
        }
        "if_statement" => guards.record(unit, lineage, GUARD_SCOPES, src),
        "binary_expression"
            if is_empty_fallback(node) && try_handling::inside_try(node, &TRY_SYNTAX) =>
        {
            unit.review_risks.empty_fallbacks += 1;
        }
        "await_expression" | "yield_expression" => guards.clear(),
        "assignment_expression" | "augmented_assignment_expression" => {
            if let Some(left) = node.field("left") {
                guards.forget_written(left, src);
            }
        }
        "update_expression" => {
            if let Some(argument) = node.field("argument") {
                guards.forget_written(argument, src);
            }
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
        .filter(|parent| parent.kind_name() == "arguments")?;
    let call = func
        .parent(args)
        .filter(|parent| parent.kind_name() == "call_expression")?;
    base_callee(call.field("function")?, src)
}

/// Identifier under `.only` / `.each` / `.concurrent` modifiers and curried calls.
fn base_callee<'a>(callee: Node<'_>, src: &'a [u8]) -> Option<&'a str> {
    match callee.kind_name() {
        "identifier" => callee.utf8_text(src).ok(),
        "call_expression" => base_callee(callee.field("function")?, src),
        "member_expression" => {
            let modifier = callee.field("property")?.utf8_text(src).ok()?;
            if !matches!(modifier, "only" | "each" | "concurrent") {
                return None;
            }
            base_callee(callee.field("object")?, src)
        }
        _ => None,
    }
}

/// A catch that neither rethrows nor uses its error beyond logging it.
fn swallows(handler: Node<'_>, src: &[u8]) -> bool {
    handler.field("body").is_some_and(|body| {
        try_handling::swallows(body, error_name(handler, src), src, &TRY_SYNTAX)
    })
}

fn error_name<'a>(handler: Node<'_>, src: &'a [u8]) -> Option<&'a str> {
    handler
        .field("parameter")
        .and_then(|parameter| parameter.utf8_text(src).ok())
}

fn sentinel_handler(
    handler: Node<'_>,
    src: &[u8],
    frame: Option<usize>,
) -> Option<SentinelHandler> {
    let body = handler.field("body")?;
    handler_end::sentinel_handler(
        handler,
        body,
        error_name(handler, src),
        src,
        &TRY_SYNTAX,
        frame,
    )
}

fn returned_literal(node: Node<'_>, src: &[u8]) -> Option<HandlerEnd> {
    let empty = match node.kind_name() {
        "null" | "undefined" | "false" => true,
        "array" | "object" if node.named_child_count() == 0 => true,
        "string" => node.named_child_count() == 0,
        "template_string" if node.named_child_count() == 0 => true,
        "number" => node.utf8_text(src) == Ok("0"),
        "true" => false,
        _ => return None,
    };
    Some(if empty {
        HandlerEnd::EmptyValue
    } else {
        HandlerEnd::DefaultLiteral
    })
}

fn is_empty_fallback(node: Node<'_>) -> bool {
    let is_fallback_operator = (0..node.child_count())
        .filter_map(|index| node.child(index))
        .any(|child| matches!(child.kind_name(), "||" | "??"));
    is_fallback_operator && node.field("right").is_some_and(is_empty_value)
}

fn is_empty_value(node: Node<'_>) -> bool {
    matches!(node.kind_name(), "null" | "array" | "object")
        && (node.kind_name() == "null" || node.named_child_count() == 0)
}
