//! Python literal membership: `x in ("a", "b")` and `x == "a" or x == "b" or x == "c"`.

use crate::profiles::node_names::NodeNames;
use crate::spine::ir::LiteralMembership;
use tree_sitter::Node;

const OPERAND_KINDS: &[&str] = &["identifier", "attribute", "call"];
/// Fewest `x == "a" or …` comparisons that read as a category list.
const MIN_CHAIN: usize = 3;

/// `x in ["a", "b"]` — an `in`/`not in` test against a literal collection
/// whose elements are all strings.
pub(super) fn literal_membership(node: Node, src: &[u8]) -> Option<LiteralMembership> {
    let has_in = node
        .children(&mut node.walk())
        .any(|c| matches!(c.kind_name(), "in" | "not in"));
    if !has_in {
        return None;
    }
    let rhs = node.named_child(node.named_child_count().saturating_sub(1))?;
    if !matches!(rhs.kind_name(), "list" | "tuple" | "set") || rhs.named_child_count() == 0 {
        return None;
    }
    let mut cursor = rhs.walk();
    let elements: Vec<Node> = rhs.named_children(&mut cursor).collect();
    if !elements.iter().all(|e| e.kind_name() == "string") {
        return None;
    }
    let operand = node
        .named_child(0)
        .filter(|lhs| OPERAND_KINDS.contains(&lhs.kind_name()))
        .and_then(|lhs| lhs.utf8_text(src).ok())
        .map(str::to_string);
    Some(LiteralMembership {
        line: node.start_position().row + 1,
        operand,
        literals: elements
            .iter()
            .filter_map(|e| e.utf8_text(src).ok())
            .map(|t| t.trim_matches(['"', '\'']).to_string())
            .collect(),
    })
}

/// `s == "a" or s == "b" or s == "c"`, recorded once at the top of the chain.
pub(super) fn or_chain(node: Node, src: &[u8]) -> Option<LiteralMembership> {
    if !is_or(node, src) || node.parent().is_some_and(|parent| is_or(parent, src)) {
        return None;
    }
    let mut terms = Vec::new();
    flatten(node, src, &mut terms);
    let comparisons: Vec<(&str, String)> = terms
        .iter()
        .map(|term| string_equality(*term, src))
        .collect::<Option<_>>()?;
    let operand = comparisons.first()?.0;
    if comparisons.len() < MIN_CHAIN || comparisons.iter().any(|(other, _)| *other != operand) {
        return None;
    }
    Some(LiteralMembership {
        line: node.start_position().row + 1,
        operand: Some(operand.to_string()),
        literals: comparisons
            .into_iter()
            .map(|(_, literal)| literal)
            .collect(),
    })
}

fn is_or(node: Node, src: &[u8]) -> bool {
    node.kind_name() == "boolean_operator"
        && node.field("operator").and_then(|op| op.utf8_text(src).ok()) == Some("or")
}

fn flatten<'t>(node: Node<'t>, src: &[u8], out: &mut Vec<Node<'t>>) {
    if !is_or(node, src) {
        out.push(node);
        return;
    }
    for side in ["left", "right"] {
        if let Some(child) = node.field(side) {
            flatten(child, src, out);
        }
    }
}

/// `x == "a"` / `"a" == x` → (`x`, `a`).
fn string_equality<'s>(node: Node, src: &'s [u8]) -> Option<(&'s str, String)> {
    let is_equality = node.kind_name() == "comparison_operator"
        && node.named_child_count() == 2
        && node
            .children(&mut node.walk())
            .any(|c| c.kind_name() == "==");
    if !is_equality {
        return None;
    }
    let (left, right) = (node.named_child(0)?, node.named_child(1)?);
    let (subject, literal) = match (left.kind_name(), right.kind_name()) {
        (_, "string") => (left, right),
        ("string", _) => (right, left),
        _ => return None,
    };
    if !matches!(subject.kind_name(), "identifier" | "attribute") {
        return None;
    }
    Some((
        subject.utf8_text(src).ok()?,
        literal
            .utf8_text(src)
            .ok()?
            .trim_matches(['"', '\''])
            .to_string(),
    ))
}
