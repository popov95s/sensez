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

use crate::spine::ir::TypeHints;
use tree_sitter::Node;

/// Record a `function_definition`'s return + parameter annotations.
pub fn record_function(node: Node, src: &[u8], hints: &mut TypeHints) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or("")
        .to_string();
    if let Some(ret) = type_text(node.child_by_field_name("return_type"), src) {
        hints.return_types.insert(name.clone(), ret);
    }
    if let Some(params) = node.child_by_field_name("parameters") {
        collect_params(params, src, &name, hints);
    }
}

fn collect_params(params: Node, src: &[u8], func: &str, hints: &mut TypeHints) {
    let mut cursor = params.walk();
    for p in params.named_children(&mut cursor) {
        if !matches!(p.kind(), "typed_parameter" | "typed_default_parameter") {
            continue;
        }
        let pname = p
            .child_by_field_name("name")
            .or_else(|| first_identifier(p))
            .and_then(|n| n.utf8_text(src).ok());
        let ptype = type_text(p.child_by_field_name("type"), src);
        if let (Some(pname), Some(ptype)) = (pname, ptype) {
            hints
                .param_types
                .insert((func.to_string(), pname.to_string()), ptype);
        }
    }
}

/// Record `x: T` and `x = T(...)` local/global type hints.
pub fn record_assignment(node: Node, src: &[u8], hints: &mut TypeHints) {
    let Some(left) = node.child_by_field_name("left") else {
        return;
    };
    let ty = type_text(node.child_by_field_name("type"), src)
        .or_else(|| instantiated_type(node.child_by_field_name("right"), src));
    let Some(ty) = ty else { return };

    if left.kind() == "identifier" {
        if let Ok(name) = left.utf8_text(src) {
            hints.var_types.insert(name.to_string(), ty);
        }
    }
}

/// If `right` is `Name(...)` / `mod.Name(...)` with a Capitalized callee, return
/// that type name (the last dotted segment).
pub(super) fn instantiated_type(right: Option<Node>, src: &[u8]) -> Option<String> {
    let call = right.filter(|r| r.kind() == "call")?;
    let func = call.child_by_field_name("function")?;
    let text = func.utf8_text(src).ok()?;
    let last = text.rsplit('.').next().unwrap_or(text);
    match last.chars().next() {
        Some(c) if c.is_uppercase() => Some(last.to_string()),
        _ => None,
    }
}

pub(super) fn type_text(node: Option<Node>, src: &[u8]) -> Option<String> {
    node.and_then(|n| n.utf8_text(src).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn first_identifier(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .find(|c| c.kind() == "identifier");
    found
}
