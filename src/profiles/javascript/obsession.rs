//! Per-node recorders for the JS/TS type/mutation-discipline smells, called
//! from the single body walk in [`super::units`]. Mirrors `lang::python::obsession`
//! against JS/TS node kinds. Collects raw structure only — thresholds and
//! severity live in `noze::smells`.

use super::fallback_sink::fallback_sink;
use super::membership;
use crate::profiles::fallback_guard::{fallback_branch, GuardSyntax};
use crate::profiles::walk::{self, MutationTarget};
use crate::spine::ir::{record_attr, FunctionUnit, MutatorFamily, SchemaCall};
use tree_sitter::Node;

/// Methods whose call mutates an array receiver in place.
const ARRAY_MUTATORS: [&str; 9] = [
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "sort",
    "reverse",
    "fill",
    "copyWithin",
];

const GUARD_SYNTAX: GuardSyntax = GuardSyntax {
    truthy_prefixes: &["!!"],
    presence_suffixes: &[
        " != null",
        " !== null",
        " != undefined",
        " !== undefined",
        " !== \"\"",
        " !== ''",
    ],
    absence_prefixes: &["!"],
    absence_suffixes: &[
        " == null",
        " === null",
        " == undefined",
        " === undefined",
        " === \"\"",
        " === ''",
    ],
    accessors: &[".", "?.", "["],
};

/// Methods whose call mutates a Map / Set style receiver in place.
const COLLECTION_MUTATORS: [&str; 5] = ["set", "add", "delete", "clear", "remove"];

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
    let Some(right) = node.child_by_field_name("right") else {
        return;
    };
    walk::record_string_fallback(
        unit,
        string_literal_text(right, src),
        fallback_sink(node, src),
        right.start_position().row + 1,
    );
}

/// `x ? x : "?"` or `!x ? "?" : x` — a ternary is a fallback only when it guards its own value.
fn record_ternary_fallback(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let (Some(condition), Some(then), Some(otherwise)) = (
        node.child_by_field_name("condition"),
        node.child_by_field_name("consequence"),
        node.child_by_field_name("alternative"),
    ) else {
        return;
    };
    let Some(fallback) = fallback_branch(condition, then, otherwise, src, &GUARD_SYNTAX) else {
        return;
    };
    walk::record_string_fallback(
        unit,
        string_literal_text(fallback, src),
        fallback_sink(node, src),
        fallback.start_position().row + 1,
    );
}

fn string_literal_text(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "parenthesized_expression" => node
            .named_child(0)
            .and_then(|child| string_literal_text(child, src)),
        "string" => quoted_string_body(node.utf8_text(src).ok()?, ['"', '\'']),
        "template_string" => template_string_body(node.utf8_text(src).ok()?),
        _ => None,
    }
}

fn quoted_string_body(text: &str, quotes: [char; 2]) -> Option<String> {
    let quote = text.chars().next().filter(|ch| quotes.contains(ch))?;
    let body = text.get(quote.len_utf8()..)?;
    Some(body.strip_suffix(quote)?.to_string())
}

fn template_string_body(text: &str) -> Option<String> {
    if text.contains("${") {
        return None;
    }
    Some(text.strip_prefix('`')?.strip_suffix('`')?.to_string())
}

/// `recv["key"]` — record the distinct string-literal key per receiver.
fn record_subscript(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let recv = node
        .child_by_field_name("object")
        .filter(|v| v.kind() == "identifier")
        .and_then(|v| v.utf8_text(src).ok());
    let index = node.child_by_field_name("index");
    if let (Some(recv), Some(index)) = (recv, index) {
        if is_computed_key(index, src) {
            unit.dynamic_key_receivers.insert(recv.to_string());
        }
    }
    let key = index
        .filter(|k| k.kind() == "string")
        .and_then(|k| k.utf8_text(src).ok());
    if let (Some(recv), Some(key)) = (recv, key) {
        record_attr(&mut unit.str_keys, recv, key.trim_matches(['"', '\'', '`']));
        walk::record_str_key_line(unit, recv, node.start_position().row + 1);
    }
}

/// `recv[cat]`, `recv[r.cat]`, `` recv[`${a}`] `` — a data-driven key, not a record field.
fn is_computed_key(index: Node, src: &[u8]) -> bool {
    match index.kind() {
        "string" | "number" => false,
        "template_string" => index.utf8_text(src).is_ok_and(|text| text.contains("${")),
        _ => true,
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
    record_schema_call(unit, node, src);
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
        Some(m) if ARRAY_MUTATORS.contains(&m) => {
            record_method_mutation(unit, object, src, MutatorFamily::Array)
        }
        Some(m) if COLLECTION_MUTATORS.contains(&m) => {
            record_method_mutation(unit, object, src, MutatorFamily::Collection)
        }
        Some("includes") => membership::record(unit, node, membership::literal_array(object), src),
        Some("has") => membership::record(unit, node, membership::literal_set(object, src), src),
        _ => {}
    }
}

fn record_method_mutation(
    unit: &mut FunctionUnit,
    object: Node,
    src: &[u8],
    family: MutatorFamily,
) {
    match target_root(object, src) {
        Some(MutationTarget::ViaAttribute(root)) => {
            unit.attr_mutated_names.insert(root);
        }
        Some(MutationTarget::Direct(root)) => {
            unit.method_mutations.insert((root, family));
        }
        None => {}
    }
}

fn record_schema_call(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    let is_schema_call = function.kind() == "identifier";
    let is_validator = function
        .child_by_field_name("property")
        .and_then(|property| property.utf8_text(src).ok())
        .is_some_and(|method| matches!(method, "parse" | "safeParse" | "decode"));
    if !is_schema_call && !is_validator {
        return;
    }

    let arguments = identifier_arguments(node, src);
    if is_schema_call && !arguments.is_empty() {
        unit.schema_calls.push(SchemaCall {
            target: function.utf8_text(src).unwrap_or_default().to_string(),
            arguments: arguments.clone(),
        });
    }
    if is_validator {
        unit.validated_names.extend(arguments);
    }
}

fn identifier_arguments(node: Node, src: &[u8]) -> Vec<String> {
    let Some(arguments) = node.child_by_field_name("arguments") else {
        return Vec::new();
    };
    let mut cursor = arguments.walk();
    arguments
        .named_children(&mut cursor)
        .filter(|argument| argument.kind() == "identifier")
        .filter_map(|argument| argument.utf8_text(src).ok().map(str::to_string))
        .collect()
}

/// Resolve `node` to its root identifier (via [`target_root`]) and record it as
/// a direct or attribute-deep mutation. See [`walk::record_mutation_root`].
fn record_root(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    walk::record_mutation_root(unit, node, src, target_root);
}

/// Root of a subscript/member chain: `d["a"]` → `Direct(d)`, `this.x` → `ViaAttribute(self)`.
fn target_root(node: Node, src: &[u8]) -> Option<MutationTarget> {
    match node.kind() {
        "identifier" => node
            .utf8_text(src)
            .ok()
            .map(|id| MutationTarget::Direct(id.to_string())),
        "this" => Some(MutationTarget::Direct("self".to_string())),
        "subscript_expression" => target_root(node.child_by_field_name("object")?, src),
        "member_expression" => {
            target_root(node.child_by_field_name("object")?, src).map(MutationTarget::via_attribute)
        }
        _ => None,
    }
}
