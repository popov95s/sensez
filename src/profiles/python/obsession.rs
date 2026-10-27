//! Per-node recorders for the type/mutation-discipline smells, called from the
//! single body walk in [`super::units`] (no second traversal). Collects raw
//! structure only — thresholds and severity live in `noze::smells`.

use super::fallback_sink::fallback_sink;
use super::membership;
use crate::profiles::fallback_guard::{fallback_branch, GuardSyntax};
use crate::profiles::node_names::NodeNames;
use crate::profiles::walk::{self, MutationTarget};
use crate::spine::ir::{FunctionUnit, SchemaCall};
use tree_sitter::Node;

const GUARD_SYNTAX: GuardSyntax = GuardSyntax {
    truthy_prefixes: &[],
    presence_suffixes: &[" is not None", " != None", " != \"\"", " != ''"],
    absence_prefixes: &["not "],
    absence_suffixes: &[" is None", " == None", " == \"\"", " == ''"],
    accessors: &[".", "["],
};

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
    match node.kind_name() {
        "subscript" => record_subscript(unit, node, src),
        "assignment" | "augmented_assignment" => {
            if let Some(left) = node.field("left") {
                record_subscript_target(unit, left, src);
            }
        }
        "delete_statement" => {
            let mut cursor = node.walk();
            for target in node.named_children(&mut cursor) {
                record_subscript_target(unit, target, src);
            }
        }
        "call" => {
            record_get_key(unit, node, src);
            record_get_default(unit, node, src);
            record_schema_call(unit, node, src);
            record_mutating_call(unit, node, src);
        }
        "return_statement" => {
            if let Some(value) = node.named_child(0) {
                if matches!(value.kind_name(), "expression_list" | "tuple") {
                    let arity = value.named_child_count();
                    unit.max_tuple_return = unit.max_tuple_return.max(arity);
                }
            }
        }
        "comparison_operator" => unit
            .literal_memberships
            .extend(membership::literal_membership(node, src)),
        "boolean_operator" => {
            record_boolean_fallback(unit, node, src);
            unit.literal_memberships
                .extend(membership::or_chain(node, src));
        }
        "conditional_expression" => record_conditional_fallback(unit, node, src),
        _ => {}
    }
}

fn record_schema_call(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let Some(function) = node.field("function") else {
        return;
    };
    let target = function.utf8_text(src).unwrap_or_default();
    let arguments = identifier_arguments(node, src);
    if function.kind_name() == "identifier" && !arguments.is_empty() {
        unit.schema_calls.push(SchemaCall {
            target: target.to_string(),
            arguments: arguments.clone(),
        });
    }
    let validator = target.rsplit('.').next().is_some_and(|method| {
        matches!(
            method,
            "model_validate" | "parse_obj" | "validate_python" | "load"
        )
    });
    if validator {
        unit.validated_names.extend(arguments);
    }
}

fn identifier_arguments(node: Node, src: &[u8]) -> Vec<String> {
    let Some(arguments) = node.field("arguments") else {
        return Vec::new();
    };
    let mut cursor = arguments.walk();
    arguments
        .named_children(&mut cursor)
        .filter(|argument| argument.kind_name() == "identifier")
        .filter_map(|argument| argument.utf8_text(src).ok().map(str::to_string))
        .collect()
}

/// `value or ""` / `value or "?"` — the fallback is Python's right operand.
fn record_boolean_fallback(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let is_or = node.field("operator").and_then(|op| op.utf8_text(src).ok()) == Some("or");
    if !is_or {
        return;
    }
    let Some(right) = node.field("right") else {
        return;
    };
    walk::record_string_fallback(
        unit,
        string_literal_text(right, src),
        fallback_sink(node, src),
        right.start_position().row + 1,
    );
}

/// `recv.get("key")` — the key counts toward `recv`'s implicit schema.
fn record_get_key(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let Some(function) = node
        .field("function")
        .filter(|function| attribute_name(*function, src) == Some("get"))
    else {
        return;
    };
    let recv = function
        .field("object")
        .filter(|object| object.kind_name() == "identifier")
        .and_then(|object| object.utf8_text(src).ok());
    let key = node
        .field("arguments")
        .and_then(|args| args.named_child(0))
        .filter(|key| key.kind_name() == "string")
        .and_then(|key| string_literal_text(key, src));
    if let (Some(recv), Some(key)) = (recv, key) {
        unit.str_keys
            .entry(recv.to_string())
            .or_default()
            .insert(key);
        walk::record_str_key_line(unit, recv, node.start_position().row + 1);
    }
}

/// `d.get("k", "")` / `getattr(o, "k", "")` — the string default stands in for a missing value.
fn record_get_default(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let (Some(function), Some(arguments)) = (node.field("function"), node.field("arguments"))
    else {
        return;
    };
    let default_index = match function.kind_name() {
        "attribute" if attribute_name(function, src) == Some("get") => 1,
        "identifier" if function.utf8_text(src).ok() == Some("getattr") => 2,
        _ => return,
    };
    let Some(default) = arguments.named_child(default_index) else {
        return;
    };
    walk::record_string_fallback(
        unit,
        string_literal_text(default, src),
        fallback_sink(node, src),
        default.start_position().row + 1,
    );
}

fn attribute_name<'a>(attribute: Node, src: &'a [u8]) -> Option<&'a str> {
    attribute
        .field("attribute")
        .and_then(|name| name.utf8_text(src).ok())
}

/// `value if value else "?"` or `"?" if value is None else value` — Python leaves
/// the branches unnamed, but the grammar fixes them as then, condition, otherwise.
fn record_conditional_fallback(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let (Some(then), Some(condition), Some(otherwise)) = (
        node.named_child(0),
        node.named_child(1),
        node.named_child(2),
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
    match node.kind_name() {
        "parenthesized_expression" => node
            .named_child(0)
            .and_then(|child| string_literal_text(child, src)),
        "string" => python_string_body(node.utf8_text(src).ok()?),
        _ => None,
    }
}

fn python_string_body(text: &str) -> Option<String> {
    let start = text.find(['"', '\''])?;
    let quoted = &text[start..];
    let quote = quoted.chars().next()?;
    let triple = quoted.starts_with(&quote.to_string().repeat(3));
    let delimiter_len = if triple { 3 } else { 1 };
    let body = quoted.get(delimiter_len..)?;
    let suffix = quote.to_string().repeat(delimiter_len);
    Some(body.strip_suffix(&suffix)?.to_string())
}

/// `recv["key"]` — record the distinct string-literal key per receiver.
fn record_subscript(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let recv = node
        .field("value")
        .filter(|v| v.kind_name() == "identifier")
        .and_then(|v| v.utf8_text(src).ok());
    let index = node.field("subscript");
    if let (Some(recv), Some(index)) = (recv, index) {
        if is_computed_key(index) {
            unit.dynamic_key_receivers.insert(recv.to_string());
        }
    }
    let key = index
        .filter(|k| k.kind_name() == "string")
        .and_then(|k| k.utf8_text(src).ok());
    if let (Some(recv), Some(key)) = (recv, key) {
        unit.str_keys
            .entry(recv.to_string())
            .or_default()
            .insert(key.trim_matches(['"', '\'']).to_string());
        walk::record_str_key_line(unit, recv, node.start_position().row + 1);
    }
}

/// `recv[cat]`, `recv[row.cat]`, `recv[f"{a}"]`: a data-driven key, not a record field.
fn is_computed_key(index: Node) -> bool {
    match index.kind_name() {
        "string" => index
            .named_children(&mut index.walk())
            .any(|c| c.kind_name() == "interpolation"),
        "integer" | "float" | "slice" | "unary_operator" | "true" | "false" | "none" => false,
        _ => true,
    }
}

/// `x.append(...)` and friends — record `x` as mutated.
fn record_mutating_call(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let Some(func) = node
        .field("function")
        .filter(|f| f.kind_name() == "attribute")
    else {
        return;
    };
    let method = func.field("attribute").and_then(|a| a.utf8_text(src).ok());
    if !method.is_some_and(|m| MUTATORS.contains(&m)) {
        return;
    }
    if let Some(object) = func.field("object") {
        record_root(unit, object, src);
    }
}

/// Record a subscript-assign / `del` target. Only subscript targets count
/// (`d[k]=v`, `m.x[k]=v`), never a bare name (`x = ...` is reassignment, not
/// mutation). Routes to the direct vs attribute-deep set by whether the path
/// to the root identifier crossed an attribute.
fn record_subscript_target(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    if node.kind_name() == "subscript" {
        record_root(unit, node, src);
    }
}

/// Resolve `node` to its root identifier (via [`target_root`]) and record it as
/// a direct or attribute-deep mutation. See [`walk::record_mutation_root`].
fn record_root(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    walk::record_mutation_root(unit, node, src, target_root);
}

/// Root of a subscript/attribute chain: `d["a"]` → `Direct(d)`, `self.cache` → `ViaAttribute(self)`.
fn target_root(node: Node, src: &[u8]) -> Option<MutationTarget> {
    match node.kind_name() {
        "identifier" => node
            .utf8_text(src)
            .ok()
            .map(|id| MutationTarget::Direct(id.to_string())),
        "subscript" => target_root(node.field("value")?, src),
        "attribute" => target_root(node.field("object")?, src).map(MutationTarget::via_attribute),
        _ => None,
    }
}
