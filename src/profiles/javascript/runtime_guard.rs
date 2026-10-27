//! Whether a non-null call result was already checked at runtime.

use crate::profiles::node_names::NodeNames;
use tree_sitter::Node;

const CONDITIONALS: &[&str] = &[
    "if_statement",
    "ternary_expression",
    "while_statement",
    "binary_expression",
];

/// `call` is checked by an enclosing condition or an earlier `has`/`length` test.
pub(super) fn is_checked(call: Node<'_>, src: &[u8]) -> bool {
    let Ok(text) = call.utf8_text(src) else {
        return false;
    };
    let witnesses = witnesses(text);
    let mut node = call;
    while let Some(parent) = node.parent() {
        if super::units::is_function(parent.kind_name()) {
            return false;
        }
        if CONDITIONALS.contains(&parent.kind_name()) && condition_mentions(parent, node, text, src)
        {
            return true;
        }
        let earlier = std::iter::successors(node.prev_named_sibling(), |n| n.prev_named_sibling());
        if earlier
            .filter_map(|sibling| sibling.utf8_text(src).ok())
            .any(|sibling| {
                witnesses
                    .iter()
                    .any(|witness| sibling.contains(witness.as_str()))
            })
        {
            return true;
        }
        node = parent;
    }
    false
}

/// `m.get(k)` → `m.has(k)`; `xs.pop()` / `xs.shift()` → `xs.length`.
fn witnesses(call: &str) -> Vec<String> {
    let lookup = call
        .rsplit_once(".get(")
        .map(|(receiver, key)| format!("{receiver}.has({key}"));
    let drain = [".pop()", ".shift()"]
        .iter()
        .find_map(|tail| call.strip_suffix(tail))
        .map(|receiver| format!("{receiver}.length"));
    lookup.into_iter().chain(drain).collect()
}

/// The condition part of `parent` (not the branch holding `child`) names `call`.
fn condition_mentions(parent: Node<'_>, child: Node<'_>, call: &str, src: &[u8]) -> bool {
    let condition = match parent.kind_name() {
        "binary_expression" => parent.field("left"),
        _ => parent.field("condition"),
    };
    condition
        .filter(|condition| condition.id() != child.id())
        .and_then(|condition| condition.utf8_text(src).ok())
        .is_some_and(|condition| {
            condition.contains(call)
                || witnesses(call)
                    .iter()
                    .any(|w| condition.contains(w.as_str()))
        })
}
