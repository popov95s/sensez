//! Where a JS/TS string fallback's value flows.

use crate::profiles::walk;
use crate::spine::ir::{FallbackSink, FallbackSite};
use tree_sitter::Node;

const TRANSPARENT: &[&str] = &[
    "parenthesized_expression",
    "as_expression",
    "satisfies_expression",
    "non_null_expression",
];
const STRING_CONSUMERS: &[&str] = &[
    "localeCompare",
    "useState",
    "String",
    "Number",
    "parseInt",
    "parseFloat",
    "parse",
    "encodeURIComponent",
    "cn",
    "clsx",
    "classNames",
    "join",
    "includes",
    "startsWith",
    "endsWith",
    "trim",
    "toLowerCase",
    "toUpperCase",
    "split",
    "padStart",
    "padEnd",
    "test",
    "exec",
    "match",
    "replace",
    "has",
    "get",
];
const TRANSLATORS: &[&str] = &["t", "translate", "formatMessage"];

pub(super) fn fallback_sink(node: Node, src: &[u8]) -> FallbackSite {
    let mut child = node;
    while let Some(parent) = child.parent() {
        if TRANSPARENT.contains(&parent.kind()) || is_ternary_branch(parent, child) {
            child = parent;
            continue;
        }
        return match parent.kind() {
            "template_substitution" | "jsx_expression" => FallbackSink::Rendered.into(),
            "member_expression" => FallbackSink::StringOp.into(),
            "arguments" => arguments_sink(parent, src),
            "binary_expression" => binary_sink(parent, src).into(),
            "pair" => pair_sink(parent, src),
            "assignment_expression" => stored_in(parent.child_by_field_name("left"), src),
            "return_statement" | "arrow_function" => FallbackSink::Returned.into(),
            _ => FallbackSink::Other.into(),
        };
    }
    FallbackSink::Other.into()
}

fn is_ternary_branch(parent: Node, child: Node) -> bool {
    parent.kind() == "ternary_expression"
        && parent
            .child_by_field_name("condition")
            .is_some_and(|condition| condition.id() != child.id())
}

fn arguments_sink(arguments: Node, src: &[u8]) -> FallbackSite {
    match callee_name(arguments, src) {
        Some(callee) if STRING_CONSUMERS.contains(&callee) => FallbackSink::StringOp.into(),
        _ if rendered_call(arguments, src) => FallbackSink::Rendered.into(),
        callee => FallbackSite::to(FallbackSink::Passed, callee.unwrap_or_default()),
    }
}

/// The call owning `arguments` is itself rendered: `${escape(x ?? "—")}`.
fn rendered_call(arguments: Node, src: &[u8]) -> bool {
    arguments
        .parent()
        .is_some_and(|call| fallback_sink(call, src).sink == FallbackSink::Rendered)
}

/// `{ key: x ?? "" }`; rendered when the object is a translation call's options.
fn pair_sink(pair: Node, src: &[u8]) -> FallbackSite {
    let translated = pair
        .parent()
        .and_then(|object| object.parent())
        .filter(|parent| parent.kind() == "arguments")
        .and_then(|arguments| callee_name(arguments, src))
        .is_some_and(|callee| TRANSLATORS.contains(&callee));
    if translated {
        return FallbackSink::Rendered.into();
    }
    stored_in(pair.child_by_field_name("key"), src)
}

fn stored_in(target: Option<Node>, src: &[u8]) -> FallbackSite {
    let name = target
        .and_then(|target| walk::node_text(target, src))
        .unwrap_or_default();
    FallbackSite::to(FallbackSink::Stored, name)
}

fn binary_sink(node: Node, src: &[u8]) -> FallbackSink {
    let operator = node
        .child_by_field_name("operator")
        .and_then(|op| walk::node_text(op, src));
    match operator {
        Some("+") => FallbackSink::Rendered,
        Some("===" | "!==" | "==" | "!=") => FallbackSink::Compared,
        _ => FallbackSink::Other,
    }
}

fn callee_name<'a>(arguments: Node, src: &'a [u8]) -> Option<&'a str> {
    let callee = arguments.parent()?.child_by_field_name("function")?;
    match callee.kind() {
        "identifier" => walk::node_text(callee, src),
        "member_expression" => walk::node_text(callee.child_by_field_name("property")?, src),
        _ => None,
    }
}
