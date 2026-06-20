//! Per-node recorders for the type/mutation-discipline smells, called from the
//! single body walk in [`super::units`] (no second traversal). Collects raw
//! structure only — thresholds and severity live in `noze::smells`.

use crate::profiles::walk;
use crate::spine::ir::FunctionUnit;
use tree_sitter::Node;

/// Method names whose call mutates the receiver in place (list/dict/set/deque).
const MUTATORS: [&str; 13] = [
    "append",
    "extend",
    "insert",
    "remove",
    "pop",
    "popitem",
    "clear",
    "sort",
    "reverse",
    "update",
    "setdefault",
    "add",
    "discard",
];

/// Record everything this node contributes to the discipline smells.
pub fn scan(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    match node.kind() {
        "subscript" => record_subscript(unit, node, src),
        "assignment" | "augmented_assignment" => {
            if let Some(left) = node.child_by_field_name("left") {
                record_subscript_target(unit, left, src);
            }
        }
        "delete_statement" => {
            let mut cursor = node.walk();
            for target in node.named_children(&mut cursor) {
                record_subscript_target(unit, target, src);
            }
        }
        "call" => record_mutating_call(unit, node, src),
        "return_statement" => {
            if let Some(value) = node.named_child(0) {
                if matches!(value.kind(), "expression_list" | "tuple") {
                    let arity = value.named_child_count();
                    unit.max_tuple_return = unit.max_tuple_return.max(arity);
                }
            }
        }
        "comparison_operator" if is_literal_membership(node) => {
            unit.literal_membership_tests += 1;
        }
        "boolean_operator" => record_boolean_fallback(unit, node, src),
        "conditional_expression" => record_conditional_fallback(unit, node, src),
        _ => {}
    }
}

/// `value or ""` / `value or "?"` — the fallback is Python's right operand.
fn record_boolean_fallback(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let is_or = node
        .child_by_field_name("operator")
        .and_then(|op| op.utf8_text(src).ok())
        == Some("or");
    if !is_or {
        return;
    }
    let len = node
        .child_by_field_name("right")
        .and_then(|right| string_literal_len(right, src));
    walk::record_magic_string_default(unit, len);
}

/// `value if condition else "?"` — Python leaves the branches unnamed, but the
/// grammar fixes the `else` branch as the last named child.
fn record_conditional_fallback(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let len = node
        .named_child(node.named_child_count().saturating_sub(1))
        .and_then(|fallback| string_literal_len(fallback, src));
    walk::record_magic_string_default(unit, len);
}

fn string_literal_len(node: Node, src: &[u8]) -> Option<usize> {
    match node.kind() {
        "parenthesized_expression" => node
            .named_child(0)
            .and_then(|child| string_literal_len(child, src)),
        "string" => python_string_len(node.utf8_text(src).ok()?),
        _ => None,
    }
}

fn python_string_len(text: &str) -> Option<usize> {
    let start = text.find(['"', '\''])?;
    let quoted = &text[start..];
    let quote = quoted.chars().next()?;
    let triple = quoted.starts_with(&quote.to_string().repeat(3));
    let delimiter_len = if triple { 3 } else { 1 };
    let body = quoted.get(delimiter_len..)?;
    let suffix = quote.to_string().repeat(delimiter_len);
    Some(body.strip_suffix(&suffix)?.chars().count())
}

/// `recv["key"]` — record the distinct string-literal key per receiver.
fn record_subscript(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let recv = node
        .child_by_field_name("value")
        .filter(|v| v.kind() == "identifier")
        .and_then(|v| v.utf8_text(src).ok());
    let key = node
        .child_by_field_name("subscript")
        .filter(|k| k.kind() == "string")
        .and_then(|k| k.utf8_text(src).ok());
    if let (Some(recv), Some(key)) = (recv, key) {
        unit.str_keys
            .entry(recv.to_string())
            .or_default()
            .insert(key.trim_matches(['"', '\'']).to_string());
    }
}

/// `x.append(...)` and friends — record `x` as mutated.
fn record_mutating_call(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let Some(func) = node
        .child_by_field_name("function")
        .filter(|f| f.kind() == "attribute")
    else {
        return;
    };
    let method = func
        .child_by_field_name("attribute")
        .and_then(|a| a.utf8_text(src).ok());
    if !method.is_some_and(|m| MUTATORS.contains(&m)) {
        return;
    }
    if let Some(object) = func.child_by_field_name("object") {
        record_root(unit, object, src);
    }
}

/// Record a subscript-assign / `del` target. Only subscript targets count
/// (`d[k]=v`, `m.x[k]=v`), never a bare name (`x = ...` is reassignment, not
/// mutation). Routes to the direct vs attribute-deep set by whether the path
/// to the root identifier crossed an attribute.
fn record_subscript_target(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    if node.kind() == "subscript" {
        record_root(unit, node, src);
    }
}

/// Resolve `node` to its root identifier (via [`target_root`]) and record it as
/// a direct or attribute-deep mutation. See [`walk::record_mutation_root`].
fn record_root(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    walk::record_mutation_root(unit, node, src, target_root);
}

/// Root identifier of a subscript/attribute chain and whether the path crossed
/// an attribute: `d["a"]["b"]` → `(d, false)`; `m.kwargs["k"]` → `(m, true)`;
/// `self.cache` → `(self, true)`. A bare identifier → `(name, false)`.
fn target_root(node: Node, src: &[u8]) -> Option<(String, bool)> {
    match node.kind() {
        "identifier" => node.utf8_text(src).ok().map(|id| (id.to_string(), false)),
        "subscript" => target_root(node.child_by_field_name("value")?, src),
        "attribute" => {
            target_root(node.child_by_field_name("object")?, src).map(|(root, _)| (root, true))
        }
        _ => None,
    }
}

/// `x in ["a", "b"]` — an `in`/`not in` test against a literal collection
/// whose elements are all strings.
fn is_literal_membership(node: Node) -> bool {
    let has_in = node
        .children(&mut node.walk())
        .any(|c| matches!(c.kind(), "in" | "not in"));
    if !has_in {
        return false;
    }
    let Some(rhs) = node.named_child(node.named_child_count().saturating_sub(1)) else {
        return false;
    };
    if !matches!(rhs.kind(), "list" | "tuple" | "set") || rhs.named_child_count() == 0 {
        return false;
    }
    let mut cursor = rhs.walk();
    let all_strings = rhs
        .named_children(&mut cursor)
        .all(|e| e.kind() == "string");
    all_strings
}
