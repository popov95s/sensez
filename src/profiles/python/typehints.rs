//! Best-effort type extraction: annotations + obvious `Name(...)` instantiation.
//!
//! No inference engine — when a type can't be read off the syntax, it's simply
//! absent, and type-assisted smells skip that target (precision over recall).
//!
//! These are per-node recorders, called from the main pre-order traversal as it
//! reaches each `function_definition`/`assignment`. Folding them in avoids a
//! second full-tree walk (the dominant FFI cost was re-descending every node):
//! the main visit already reaches exactly these nodes in the same pre-order, so
//! the collected hints are identical, one traversal cheaper.

use crate::profiles::node_names::NodeNames;
use crate::profiles::walk;
use crate::spine::ir::{FunctionKey, TypeAlias, TypeHints};
use tree_sitter::Node;

/// Record a `function_definition`'s return + parameter annotations.
pub fn record_function(node: Node, src: &[u8], hints: &mut TypeHints) {
    let key = FunctionKey {
        name: node
            .field("name")
            .and_then(|n| n.utf8_text(src).ok())
            .unwrap_or("")
            .to_string(),
        start_line: node.start_position().row + 1,
    };
    if let Some(params) = node.field("parameters") {
        collect_params(params, src, &key, hints);
    }
    if let Some(ret) = type_text(node.field("return_type"), src) {
        hints.record_return(key, ret);
    }
}

fn collect_params(params: Node, src: &[u8], func: &FunctionKey, hints: &mut TypeHints) {
    let mut cursor = params.walk();
    for p in params.named_children(&mut cursor) {
        if !matches!(p.kind_name(), "typed_parameter" | "typed_default_parameter") {
            continue;
        }
        let pname = p
            .field("name")
            .or_else(|| param_identifier(p))
            .and_then(|n| n.utf8_text(src).ok());
        let ptype = type_text(p.field("type"), src);
        if let (Some(pname), Some(ptype)) = (pname, ptype) {
            hints.record_param(func, pname, ptype);
        }
    }
}

/// Record `x: T` and `x = T(...)` local/global type hints.
pub fn record_assignment(node: Node, src: &[u8], hints: &mut TypeHints) {
    let Some(left) = node.field("left") else {
        return;
    };
    let ty =
        type_text(node.field("type"), src).or_else(|| instantiated_type(node.field("right"), src));
    let Some(ty) = ty else { return };

    if left.kind_name() == "identifier" {
        if let Ok(name) = left.utf8_text(src) {
            hints.var_types.insert(name.to_string(), ty);
        }
    } else if left.kind_name() == "attribute" {
        if let Ok(name) = left.utf8_text(src) {
            hints.attr_types.insert(name.to_string(), ty);
        }
    }
}

pub fn record_type_alias(node: Node, src: &[u8], hints: &mut TypeHints) {
    let Some((name, target)) = assignment_alias(node, src).or_else(|| pep695_alias(node, src))
    else {
        return;
    };
    hints.type_aliases.push(TypeAlias {
        name,
        target,
        line: node.start_position().row + 1,
    });
}

fn assignment_alias(node: Node, src: &[u8]) -> Option<(String, String)> {
    let left = node.field("left")?;
    if left.kind_name() != "identifier" {
        return None;
    }
    let name = left.utf8_text(src).ok()?.to_string();
    let target = node
        .field("right")
        .and_then(|n| n.utf8_text(src).ok())?
        .trim()
        .to_string();
    alias_target(&target).then_some((name, target))
}

fn pep695_alias(node: Node, src: &[u8]) -> Option<(String, String)> {
    if node.kind_name() != "type_alias_statement" {
        return None;
    }
    let text = node.utf8_text(src).ok()?.trim();
    let rest = text.strip_prefix("type ")?;
    let (name, target) = rest.split_once('=')?;
    Some((name.trim().to_string(), target.trim().to_string()))
}

/// If `right` is `Name(...)` / `mod.Name(...)` with a Capitalized callee, return
/// that type name (the last dotted segment).
pub(super) fn instantiated_type(right: Option<Node>, src: &[u8]) -> Option<String> {
    let call = right.filter(|r| r.kind_name() == "call")?;
    let func = call.field("function")?;
    let text = func.utf8_text(src).ok()?;
    let last = walk::last_segment(text);
    match last.chars().next() {
        Some(c) if c.is_uppercase() => Some(last.to_string()),
        _ => None,
    }
}

fn alias_target(target: &str) -> bool {
    target.contains('[')
        || target.contains('|')
        || matches!(
            target,
            "Any" | "str" | "int" | "float" | "bool" | "dict" | "list" | "tuple" | "set"
        )
}

pub(super) fn type_text(node: Option<Node>, src: &[u8]) -> Option<String> {
    node.and_then(|n| n.utf8_text(src).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// The name of a parameter node, looking through `*args` / `**options` patterns.
pub(super) fn param_identifier(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .find_map(|child| match child.kind_name() {
            "identifier" => Some(child),
            "list_splat_pattern" | "dictionary_splat_pattern" => param_identifier(child),
            _ => None,
        });
    found
}
