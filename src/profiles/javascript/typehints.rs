//! Best-effort TypeScript type extraction: parameter/return annotations and
//! obvious `new Foo()` instantiations. Unknown types stay absent, and
//! type-assisted smells skip those targets. JavaScript files usually contribute
//! only obvious instantiations.

use super::symbols;
use crate::spine::ir::TypeHints;
use tree_sitter::Node;

/// Record a function/method's return + parameter type annotations.
pub fn record_function(node: Node, src: &[u8], hints: &mut TypeHints) {
    let Some(name) = symbols::def_name(node, src) else {
        return; // anonymous arrow/expression — nothing to key on
    };
    if let Some(ret) = type_text(node.child_by_field_name("return_type"), src) {
        hints.return_types.insert(name.clone(), ret);
    }
    if let Some(params) = node.child_by_field_name("parameters") {
        let mut cursor = params.walk();
        for p in params.named_children(&mut cursor) {
            if !matches!(p.kind(), "required_parameter" | "optional_parameter") {
                continue;
            }
            let pname = p
                .child_by_field_name("pattern")
                .filter(|n| n.kind() == "identifier")
                .and_then(|n| n.utf8_text(src).ok());
            let ptype = type_text(p.child_by_field_name("type"), src);
            if let (Some(pname), Some(ptype)) = (pname, ptype) {
                hints
                    .param_types
                    .insert((name.clone(), pname.to_string()), ptype);
            }
        }
    }
}

/// Record `const x: T = …` / `const x = new T()` from a `variable_declarator`.
pub fn record_declaration(node: Node, src: &[u8], hints: &mut TypeHints) {
    let Some(name) = node
        .child_by_field_name("name")
        .filter(|n| n.kind() == "identifier")
        .and_then(|n| n.utf8_text(src).ok())
    else {
        return;
    };
    let ty = type_text(node.child_by_field_name("type"), src)
        .or_else(|| instantiated_type(node.child_by_field_name("value"), src));
    if let Some(ty) = ty {
        hints.var_types.insert(name.to_string(), ty);
    }
}

/// If `value` is `new Name(...)` / `new mod.Name(...)`, return that type name.
fn instantiated_type(value: Option<Node>, src: &[u8]) -> Option<String> {
    let new_expr = value.filter(|v| v.kind() == "new_expression")?;
    let ctor = new_expr.child_by_field_name("constructor")?;
    let text = ctor.utf8_text(src).ok()?;
    let last = text.rsplit('.').next().unwrap_or(text);
    match last.chars().next() {
        Some(c) if c.is_uppercase() => Some(last.to_string()),
        _ => None,
    }
}

/// Text of a `type_annotation` (`: Foo`) with the leading colon stripped.
fn type_text(node: Option<Node>, src: &[u8]) -> Option<String> {
    let text = node.and_then(|n| n.utf8_text(src).ok())?;
    let trimmed = text.trim_start_matches(':').trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
