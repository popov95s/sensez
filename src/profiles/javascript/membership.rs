//! Literal membership tests: `["a", "b"].includes(x)` and `new Set(["a"]).has(x)`.

use super::typevocab::guarded_param;
use super::units::is_function;
use crate::profiles::walk;
use crate::spine::ir::{FunctionUnit, LiteralMembership};
use tree_sitter::Node;

const OPERAND_KINDS: &[&str] = &["identifier", "member_expression", "call_expression"];
const EQUALITY: &[&str] = &["===", "=="];
/// Fewest `x === "a" || …` comparisons that read as a category list.
const MIN_CHAIN: usize = 3;

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
        .filter(|a| OPERAND_KINDS.contains(&a.kind()))
        .and_then(|a| a.utf8_text(src).ok())
        .map(str::to_string);
    if operand
        .as_deref()
        .is_some_and(|operand| guarded_by_predicate(call, operand, src))
    {
        return;
    }
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

/// `(v): v is Grade => [...].includes(v)`: the list defines the guarded type.
fn guarded_by_predicate(call: Node, operand: &str, src: &[u8]) -> bool {
    let function = std::iter::successors(call.parent(), |node| node.parent())
        .find(|node| is_function(node.kind()));
    function
        .and_then(|function| function.child_by_field_name("return_type"))
        .and_then(|annotation| walk::node_text(annotation, src))
        .and_then(|text| guarded_param(text.trim_start_matches(':').trim()))
        == Some(operand)
}

/// `s === "a" || s === "b" || s === "c"`, recorded once at the top of the chain.
pub(super) fn record_or_chain(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let is_or = |node: Node| {
        node.kind() == "binary_expression"
            && node
                .child_by_field_name("operator")
                .and_then(|op| walk::node_text(op, src))
                == Some("||")
    };
    if !is_or(node) || node.parent().is_some_and(is_or) {
        return;
    }
    let mut terms = Vec::new();
    flatten(node, &is_or, &mut terms);
    let comparisons: Option<Vec<(&str, String)>> = terms
        .iter()
        .map(|term| string_equality(*term, src))
        .collect();
    let Some(comparisons) = comparisons.filter(|found| found.len() >= MIN_CHAIN) else {
        return;
    };
    let operand = comparisons[0].0;
    if comparisons.iter().any(|(other, _)| *other != operand) {
        return;
    }
    unit.literal_memberships.push(LiteralMembership {
        line: node.start_position().row + 1,
        operand: Some(operand.to_string()),
        literals: comparisons
            .into_iter()
            .map(|(_, literal)| literal)
            .collect(),
    });
}

fn flatten<'t>(node: Node<'t>, is_or: &dyn Fn(Node) -> bool, out: &mut Vec<Node<'t>>) {
    if !is_or(node) {
        out.push(node);
        return;
    }
    for side in ["left", "right"] {
        if let Some(child) = node.child_by_field_name(side) {
            flatten(child, is_or, out);
        }
    }
}

/// `x === "a"` / `"a" === x` → (`x`, `a`).
fn string_equality<'s>(node: Node, src: &'s [u8]) -> Option<(&'s str, String)> {
    let operator = walk::node_text(node.child_by_field_name("operator")?, src)?;
    if node.kind() != "binary_expression" || !EQUALITY.contains(&operator) {
        return None;
    }
    let left = node.child_by_field_name("left")?;
    let right = node.child_by_field_name("right")?;
    let (subject, literal) = match (left.kind(), right.kind()) {
        (_, "string") => (left, right),
        ("string", _) => (right, left),
        _ => return None,
    };
    if !matches!(subject.kind(), "identifier" | "member_expression") {
        return None;
    }
    let text = walk::node_text(literal, src)?;
    Some((
        walk::node_text(subject, src)?,
        text.trim_matches(['"', '\'']).to_string(),
    ))
}
