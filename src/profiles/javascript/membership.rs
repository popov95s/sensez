//! Literal membership tests: `["a", "b"].includes(x)` and `new Set(["a"]).has(x)`.

use crate::spine::ir::{FunctionUnit, LiteralMembership};
use tree_sitter::Node;

/// `["a", "b"].includes(x)` / `new Set(["a"]).has(x)` against inline string literals.
pub(super) fn record(unit: &mut FunctionUnit, call: Node, list: Option<Node>, src: &[u8]) {
    let Some(list) = list else {
        return;
    };
    let mut cursor = list.walk();
    let literals = list
        .named_children(&mut cursor)
        .filter_map(|e| e.utf8_text(src).ok())
        .map(|t| t.trim_matches(['"', '\'']).to_string())
        .collect();
    let operand = call
        .child_by_field_name("arguments")
        .and_then(|args| args.named_child(0))
        .filter(|a| matches!(a.kind(), "identifier" | "member_expression"))
        .and_then(|a| a.utf8_text(src).ok())
        .map(str::to_string);
    unit.literal_memberships.push(LiteralMembership {
        line: call.start_position().row + 1,
        operand,
        literals,
    });
}

/// The string array behind `[...]` or `([...] as const)`.
pub(super) fn literal_array(node: Node) -> Option<Node> {
    let inner = unparenthesized(node)?;
    let array = match inner.kind() {
        "as_expression" => inner.named_child(0)?,
        _ => inner,
    };
    is_string_array(array).then_some(array)
}

/// The string array behind an inline `new Set([...])`.
pub(super) fn literal_set<'t>(node: Node<'t>, src: &[u8]) -> Option<Node<'t>> {
    let inner = unparenthesized(node)?;
    let is_set = inner.kind() == "new_expression"
        && inner
            .child_by_field_name("constructor")
            .and_then(|c| c.utf8_text(src).ok())
            == Some("Set");
    let array = inner.child_by_field_name("arguments")?.named_child(0)?;
    (is_set && is_string_array(array)).then_some(array)
}

/// True for `["a", "b", ...]` — a non-empty array literal of only strings.
fn is_string_array(node: Node) -> bool {
    if node.kind() != "array" || node.named_child_count() == 0 {
        return false;
    }
    let mut cursor = node.walk();
    let all_strings = node
        .named_children(&mut cursor)
        .all(|e| e.kind() == "string");
    all_strings
}

fn unparenthesized(node: Node) -> Option<Node> {
    match node.kind() {
        "parenthesized_expression" => node.named_child(0),
        _ => Some(node),
    }
}
