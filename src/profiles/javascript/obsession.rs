//! Per-node recorders for the JS/TS type/mutation-discipline smells, called
//! from the single body walk in [`super::units`]. Mirrors `lang::python::obsession`
//! against JS/TS node kinds. Collects raw structure only — thresholds and
//! severity live in `noze::smells`.

use crate::profiles::walk;
use crate::spine::ir::FunctionUnit;
use tree_sitter::Node;

/// Methods whose call mutates the receiver in place (Array / Map / Set).
const MUTATORS: [&str; 14] = [
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "sort",
    "reverse",
    "fill",
    "copyWithin",
    "set",
    "add",
    "delete",
    "clear",
    "remove",
];

/// Record everything this node contributes to the discipline smells.
pub fn scan(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    match node.kind() {
        "subscript_expression" => record_subscript(unit, node, src),
        "assignment_expression" | "augmented_assignment_expression" => {
            if let Some(left) = node.child_by_field_name("left") {
                if left.kind() == "subscript_expression" {
                    record_root(unit, left, src);
                }
            }
        }
        "unary_expression" => record_delete(unit, node, src),
        "call_expression" => record_call(unit, node, src),
        "binary_expression" => record_binary_fallback(unit, node, src),
        "ternary_expression" => record_ternary_fallback(unit, node, src),
        _ => {}
    }
}

/// `value || ""` / `value ?? "?"` — the fallback is JS/TS's right operand.
fn record_binary_fallback(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let is_fallback = node
        .child_by_field_name("operator")
        .and_then(|op| op.utf8_text(src).ok())
        .is_some_and(|op| matches!(op, "||" | "??"));
    if !is_fallback {
        return;
    }
    let len = node
        .child_by_field_name("right")
        .and_then(|right| string_literal_len(right, src));
    walk::record_magic_string_default(unit, len);
}

/// `condition ? value : "?"` — the fallback is the named `alternative` branch.
fn record_ternary_fallback(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let len = node
        .child_by_field_name("alternative")
        .and_then(|fallback| string_literal_len(fallback, src));
    walk::record_magic_string_default(unit, len);
}

fn string_literal_len(node: Node, src: &[u8]) -> Option<usize> {
    match node.kind() {
        "parenthesized_expression" => node
            .named_child(0)
            .and_then(|child| string_literal_len(child, src)),
        "string" => quoted_string_len(node.utf8_text(src).ok()?, ['"', '\'']),
        "template_string" => template_string_len(node.utf8_text(src).ok()?),
        _ => None,
    }
}

fn quoted_string_len(text: &str, quotes: [char; 2]) -> Option<usize> {
    let quote = text.chars().next().filter(|ch| quotes.contains(ch))?;
    let body = text.get(quote.len_utf8()..)?;
    Some(body.strip_suffix(quote)?.chars().count())
}

fn template_string_len(text: &str) -> Option<usize> {
    if text.contains("${") {
        return None;
    }
    Some(text.strip_prefix('`')?.strip_suffix('`')?.chars().count())
}

/// `recv["key"]` — record the distinct string-literal key per receiver.
fn record_subscript(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let recv = node
        .child_by_field_name("object")
        .filter(|v| v.kind() == "identifier")
        .and_then(|v| v.utf8_text(src).ok());
    let key = node
        .child_by_field_name("index")
        .filter(|k| k.kind() == "string")
        .and_then(|k| k.utf8_text(src).ok());
    if let (Some(recv), Some(key)) = (recv, key) {
        unit.str_keys
            .entry(recv.to_string())
            .or_default()
            .insert(key.trim_matches(['"', '\'', '`']).to_string());
    }
}

/// `delete obj[k]` / `delete obj.x` — record the mutated root.
fn record_delete(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let is_delete = node
        .child_by_field_name("operator")
        .and_then(|o| o.utf8_text(src).ok())
        == Some("delete");
    if !is_delete {
        return;
    }
    if let Some(arg) = node
        .child_by_field_name("argument")
        .filter(|a| matches!(a.kind(), "subscript_expression" | "member_expression"))
    {
        record_root(unit, arg, src);
    }
}

/// `x.push(...)` (mutating call) and `["a","b"].includes(y)` (literal membership).
fn record_call(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let Some(func) = node
        .child_by_field_name("function")
        .filter(|f| f.kind() == "member_expression")
    else {
        return;
    };
    let method = func
        .child_by_field_name("property")
        .and_then(|a| a.utf8_text(src).ok());
    let Some(object) = func.child_by_field_name("object") else {
        return;
    };
    match method {
        Some(m) if MUTATORS.contains(&m) => record_root(unit, object, src),
        Some("includes") if is_string_array(object) => unit.literal_membership_tests += 1,
        _ => {}
    }
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

/// Resolve `node` to its root identifier (via [`target_root`]) and record it as
/// a direct or attribute-deep mutation. See [`walk::record_mutation_root`].
fn record_root(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    walk::record_mutation_root(unit, node, src, target_root);
}

/// Root identifier of a subscript/member chain and whether it crossed a member:
/// `d["a"]["b"]` → `(d, false)`; `m.items.push` → `(m, true)`; `this.x` → `(self, true)`.
fn target_root(node: Node, src: &[u8]) -> Option<(String, bool)> {
    match node.kind() {
        "identifier" => node.utf8_text(src).ok().map(|id| (id.to_string(), false)),
        "this" => Some(("self".to_string(), false)),
        "subscript_expression" => target_root(node.child_by_field_name("object")?, src),
        "member_expression" => {
            target_root(node.child_by_field_name("object")?, src).map(|(root, _)| (root, true))
        }
        _ => None,
    }
}
