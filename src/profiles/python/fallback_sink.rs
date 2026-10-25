//! Where a Python string fallback's value flows.

use crate::profiles::walk;
use crate::spine::ir::{FallbackSink, FallbackSite};
use tree_sitter::Node;

const STRING_CONSUMERS: &[&str] = &[
    "int",
    "float",
    "str",
    "len",
    "bool",
    "Decimal",
    "fromisoformat",
    "strip",
    "lstrip",
    "rstrip",
    "lower",
    "upper",
    "split",
    "join",
    "startswith",
    "endswith",
    "replace",
    "match",
    "fullmatch",
    "search",
    "get",
    "quote",
];
const RENDERERS: &[&str] = &["print", "format", "echo", "write"];

pub(super) fn fallback_sink(node: Node, src: &[u8]) -> FallbackSite {
    let mut child = node;
    while let Some(parent) = child.parent() {
        if parent.kind() == "parenthesized_expression" || is_conditional_branch(parent, child) {
            child = parent;
            continue;
        }
        return match parent.kind() {
            "interpolation" => FallbackSink::Rendered.into(),
            "attribute" => FallbackSink::StringOp.into(),
            "argument_list" => arguments_sink(parent, src),
            "keyword_argument" => parent
                .parent()
                .map_or(FallbackSink::Other.into(), |args| arguments_sink(args, src)),
            "binary_operator" => FallbackSink::Rendered.into(),
            "comparison_operator" => FallbackSink::Compared.into(),
            "pair" => stored_in(parent.child_by_field_name("key"), src),
            "assignment" | "augmented_assignment" => {
                stored_in(parent.child_by_field_name("left"), src)
            }
            "return_statement" | "lambda" => FallbackSink::Returned.into(),
            _ => FallbackSink::Other.into(),
        };
    }
    FallbackSink::Other.into()
}

/// `then` and `otherwise` of `then if condition else otherwise`.
fn is_conditional_branch(parent: Node, child: Node) -> bool {
    parent.kind() == "conditional_expression"
        && parent
            .named_child(1)
            .is_some_and(|condition| condition.id() != child.id())
}

fn arguments_sink(arguments: Node, src: &[u8]) -> FallbackSite {
    match callee_name(arguments, src) {
        Some(callee) if STRING_CONSUMERS.contains(&callee) => FallbackSink::StringOp.into(),
        Some(callee) if RENDERERS.contains(&callee) => FallbackSink::Rendered.into(),
        callee => FallbackSite::to(FallbackSink::Passed, callee.unwrap_or_default()),
    }
}

fn stored_in(target: Option<Node>, src: &[u8]) -> FallbackSite {
    let name = target
        .and_then(|target| walk::node_text(target, src))
        .map(|text| text.trim_matches(['"', '\'']))
        .unwrap_or_default();
    FallbackSite::to(FallbackSink::Stored, name)
}

fn callee_name<'a>(arguments: Node, src: &'a [u8]) -> Option<&'a str> {
    let callee = arguments.parent()?.child_by_field_name("function")?;
    match callee.kind() {
        "identifier" => walk::node_text(callee, src),
        "attribute" => walk::node_text(callee.child_by_field_name("attribute")?, src),
        _ => None,
    }
}
