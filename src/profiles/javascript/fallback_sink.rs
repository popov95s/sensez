//! Where a JS/TS string fallback's value flows.

use crate::profiles::walk;
use crate::spine::ir::FallbackSink;
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
];

pub(super) fn fallback_sink(node: Node, src: &[u8]) -> FallbackSink {
    let mut child = node;
    while let Some(parent) = child.parent() {
        if TRANSPARENT.contains(&parent.kind()) {
            child = parent;
            continue;
        }
        return match parent.kind() {
            "template_substitution" | "jsx_expression" => FallbackSink::Rendered,
            "member_expression" => FallbackSink::StringOp,
            "arguments"
                if callee_name(parent, src).is_some_and(|c| STRING_CONSUMERS.contains(&c)) =>
            {
                FallbackSink::StringOp
            }
            "binary_expression" => binary_sink(parent, src),
            "pair" | "assignment_expression" | "arguments" => FallbackSink::Stored,
            "return_statement" | "arrow_function" => FallbackSink::Returned,
            _ => FallbackSink::Other,
        };
    }
    FallbackSink::Other
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
