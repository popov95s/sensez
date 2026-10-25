//! String defaults (`{ s = "" }`, `name = ""`) that the same function compares to that literal.

use super::units::is_function;
use crate::profiles::walk;
use crate::spine::ir::{FallbackSink, StringFallback};
use tree_sitter::Node;

const EQUALITY: &[&str] = &["===", "!==", "==", "!="];

pub(super) fn compared_defaults(func: Node, src: &[u8]) -> Vec<StringFallback> {
    let mut defaults = Vec::new();
    let mut comparisons = Vec::new();
    let mut cursor = func.walk();
    for child in func.named_children(&mut cursor) {
        collect(child, src, &mut defaults, &mut comparisons);
    }
    defaults
        .into_iter()
        .filter(|(name, literal, _)| comparisons.contains(&(name.clone(), literal.clone())))
        .map(|(_, literal, line)| StringFallback {
            line,
            literal,
            sink: FallbackSink::Compared,
            target: String::new(),
        })
        .collect()
}

type Default = (String, String, usize);
type Comparison = (String, String);

fn collect(node: Node, src: &[u8], defaults: &mut Vec<Default>, comparisons: &mut Vec<Comparison>) {
    if is_function(node.kind()) {
        return;
    }
    defaults.extend(string_default(node, src));
    comparisons.extend(literal_comparison(node, src));
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(child, src, defaults, comparisons);
    }
}

fn string_default(node: Node, src: &[u8]) -> Option<Default> {
    let (name, value) = match node.kind() {
        "object_assignment_pattern" | "assignment_pattern" => (
            node.child_by_field_name("left")?,
            node.child_by_field_name("right")?,
        ),
        "required_parameter" | "optional_parameter" => (
            node.child_by_field_name("pattern")?,
            node.child_by_field_name("value")?,
        ),
        _ => return None,
    };
    let name = walk::node_text(name, src)?;
    let literal = string_body(value, src)?;
    Some((name.to_string(), literal, node.start_position().row + 1))
}

/// `name === "literal"` in either order.
fn literal_comparison(node: Node, src: &[u8]) -> Option<Comparison> {
    if node.kind() != "binary_expression" {
        return None;
    }
    let operator = walk::node_text(node.child_by_field_name("operator")?, src)?;
    if !EQUALITY.contains(&operator) {
        return None;
    }
    let left = node.child_by_field_name("left")?;
    let right = node.child_by_field_name("right")?;
    let (name, literal) = match (left.kind(), right.kind()) {
        ("identifier", "string") => (left, right),
        ("string", "identifier") => (right, left),
        _ => return None,
    };
    Some((
        walk::node_text(name, src)?.to_string(),
        string_body(literal, src)?,
    ))
}

fn string_body(node: Node, src: &[u8]) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }
    let text = walk::node_text(node, src)?;
    Some(text.get(1..text.len().checked_sub(1)?)?.to_string())
}
