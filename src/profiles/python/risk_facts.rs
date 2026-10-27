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
    notifiers: &["flash", "messages.error", "messages.warning"],
    is_function: super::control_shapes::is_function,
    return_kind: "return_statement",
    noop_kinds: &["pass_statement"],
    comment_kind: "comment",
    cleanup_kinds: &["finally_clause"],
    parse_calls: &["json.loads", "json.load"],
    probes: &[
        "os.stat",
        "*.stat",
        "os.access",
        "importlib.import_module",
        "__import__",
    ],
    returned_literal,
};

const BROAD_TYPES: &[&str] = &["Exception", "BaseException"];

pub(super) fn forward_target(func: Node<'_>, src: &[u8], params: &[String]) -> Option<String> {
    if func
        .parent()
        .is_some_and(|parent| parent.kind_name() == "decorated_definition")
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
    if call.kind_name() != "call" {
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

pub(super) fn scan(
    unit: &mut FunctionUnit,
    guards: &mut GuardLog,
    node: Node,
    src: &[u8],
    frame: Option<usize>,
) {
    super::echo_facts::scan(unit, node, src);
    if test_oracle::is_check(node, src, &TEST_CHECKS) {
        unit.review_risks.test_checks += 1;
    }
    if let Some(helper) = test_oracle::helper_call(node, src, &TEST_CHECKS) {
        unit.review_risks.helper_calls.push(helper.to_string());
    }
    match node.kind_name() {
        "except_clause" => {
            handler(unit, node, src);
            unit.review_risks
                .sentinel_handlers
                .extend(sentinel_handler(node, src, frame));
        }
        "if_statement" => guards.record(unit, Lineage::detached(node), &["block"], src),
        "boolean_operator"
            if is_empty_fallback(node) && try_handling::inside_try(node, &TRY_SYNTAX) =>
        {
            unit.review_risks.empty_fallbacks += 1;
        }
        "await" | "yield" => guards.clear(),
        "assignment" | "augmented_assignment" => {
            if let Some(left) = node.field("left") {
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
        .filter(|child| child.kind_name() == "block");
    if bare_body.is_some_and(|body| try_handling::swallows(body, None, src, &TRY_SYNTAX)) {
        unit.review_risks.broad_handlers += 1;
    }
}

/// A bare or `Exception`-wide handler that swallows its error.
fn sentinel_handler(
    handler: Node<'_>,
    src: &[u8],
    frame: Option<usize>,
) -> Option<SentinelHandler> {
    let (caught, error) = caught_and_alias(handler, src);
    if caught.is_some_and(|caught| !BROAD_TYPES.contains(&caught)) {
        return None;
    }
    let mut cursor = handler.walk();
    let body = handler
        .named_children(&mut cursor)
        .find(|child| child.kind_name() == "block")?;
    handler_end::sentinel_handler(handler, body, error, src, &TRY_SYNTAX, frame)
}

/// Caught type text and bound name: `except ValueError as error` → (`ValueError`, `error`).
fn caught_and_alias<'a>(handler: Node<'_>, src: &'a [u8]) -> (Option<&'a str>, Option<&'a str>) {
    let text = |node: Node<'_>| node.utf8_text(src).ok();
    let Some(value) = handler.field("value") else {
        return (None, None);
    };
    if value.kind_name() != "as_pattern" {
        let alias = handler.field("alias").and_then(text);
        return (text(value), alias);
    }
    let alias = value
        .field("alias")
        .and_then(|target| target.named_child(0).or(Some(target)))
        .and_then(text);
    (value.named_child(0).and_then(text), alias)
}

fn returned_literal(node: Node<'_>, src: &[u8]) -> Option<HandlerEnd> {
    let empty = match node.kind_name() {
        "none" | "false" => true,
        "list" | "dictionary" | "tuple" if node.named_child_count() == 0 => true,
        "string" if has_child(node, "interpolation") => return None,
        "string" => !has_child(node, "string_content"),
        "integer" => node.utf8_text(src) == Ok("0"),
        "true" | "float" => false,
        _ => return None,
    };
    Some(if empty {
        HandlerEnd::EmptyValue
    } else {
        HandlerEnd::DefaultLiteral
    })
}

fn has_child(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| child.kind_name() == kind);
    found
}

fn is_empty_fallback(node: Node<'_>) -> bool {
    let is_or = (0..node.child_count())
        .filter_map(|index| node.child(index))
        .any(|child| child.kind_name() == "or");
    is_or
        && node
            .named_child(node.named_child_count().saturating_sub(1))
            .is_some_and(is_empty_value)
}

fn is_empty_value(node: Node<'_>) -> bool {
    matches!(node.kind_name(), "none" | "list" | "dictionary")
        && (node.kind_name() == "none" || node.named_child_count() == 0)
}
