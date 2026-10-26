//! Best-effort TypeScript type extraction: parameter/return annotations and
//! obvious `new Foo()` instantiations. Unknown types stay absent, and
//! type-assisted smells skip those targets. JavaScript files usually contribute
//! only obvious instantiations.

use super::symbols;
use crate::profiles::lineage::Lineage;
use crate::profiles::typevocab::has_token;
use crate::spine::ir::{FunctionKey, TypeAlias, TypeHints};
use tree_sitter::Node;

/// Record a function/method's return + parameter type annotations.
pub fn record_function(lineage: Lineage, src: &[u8], hints: &mut TypeHints) {
    let node = lineage.node();
    let Some(name) = symbols::function_name(lineage, src) else {
        return;
    };
    let key = FunctionKey {
        name,
        start_line: node.start_position().row + 1,
    };
    let earlier = earlier_siblings(lineage, declaration(lineage));
    if is_overload_implementation(&earlier, &key.name, src) {
        return; // the overload signatures carry the contract
    }
    let any_allowed = follows_any_directive(earlier.first().copied(), src);
    let ret = type_text(node.child_by_field_name("return_type"), src)
        .filter(|ty| !(any_allowed && has_token(ty, "any")));
    if let Some(ret) = ret {
        hints.record_return(key.clone(), ret);
    }
    if let Some(params) = node.child_by_field_name("parameters") {
        let mut cursor = params.walk();
        let mut previous = None;
        for p in params.named_children(&mut cursor) {
            let before = previous.replace(p);
            if !matches!(p.kind(), "required_parameter" | "optional_parameter") {
                continue;
            }
            let pname = p
                .child_by_field_name("pattern")
                .filter(|n| n.kind() == "identifier")
                .and_then(|n| n.utf8_text(src).ok());
            let param_any_allowed = any_allowed || follows_any_directive(before, src);
            let ptype = type_text(p.child_by_field_name("type"), src)
                .filter(|ty| !(param_any_allowed && has_token(ty, "any")));
            if let (Some(pname), Some(ptype)) = (pname, ptype) {
                hints.record_param(&key, pname, ptype);
            }
        }
    }
}

/// The statement a function sits in: its `export` wrapper when exported.
fn declaration<'tree>(lineage: Lineage<'_, 'tree>) -> Node<'tree> {
    lineage
        .parent(lineage.node())
        .filter(|p| p.kind() == "export_statement")
        .unwrap_or(lineage.node())
}

/// Named siblings before `node`, nearest first.
fn earlier_siblings<'tree>(lineage: Lineage<'_, 'tree>, node: Node<'tree>) -> Vec<Node<'tree>> {
    let Some(parent) = lineage.parent(node) else {
        return Vec::new();
    };
    let mut cursor = parent.walk();
    let mut earlier: Vec<Node> = parent
        .named_children(&mut cursor)
        .take_while(|sibling| sibling.id() != node.id())
        .collect();
    earlier.reverse();
    earlier
}

/// Nearest earlier sibling that is not a comment, unwrapping `export`.
fn previous_declaration<'tree>(earlier: &[Node<'tree>]) -> Option<Node<'tree>> {
    let prev = earlier.iter().copied().find(|n| n.kind() != "comment")?;
    if prev.kind() == "export_statement" {
        return prev.child_by_field_name("declaration");
    }
    Some(prev)
}

/// An implementation signature preceded by same-named overload signatures.
fn is_overload_implementation(earlier: &[Node], name: &str, src: &[u8]) -> bool {
    previous_declaration(earlier).is_some_and(|prev| {
        matches!(
            prev.kind(),
            "function_signature" | "method_signature" | "abstract_method_signature"
        ) && symbols::def_name(prev, src).as_deref() == Some(name)
    })
}

/// `// eslint-disable-next-line @typescript-eslint/no-explicit-any` as the sibling right above.
fn follows_any_directive(previous: Option<Node>, src: &[u8]) -> bool {
    previous
        .filter(|prev| prev.kind() == "comment")
        .and_then(|prev| prev.utf8_text(src).ok())
        .is_some_and(|text| text.contains("eslint-disable") && text.contains("no-explicit-any"))
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

pub fn record_type_alias(node: Node, src: &[u8], hints: &mut TypeHints) {
    let Some((name, target)) = parse_type_alias(node, src) else {
        return;
    };
    hints.type_aliases.push(TypeAlias {
        name,
        target,
        line: node.start_position().row + 1,
    });
}

fn parse_type_alias(node: Node, src: &[u8]) -> Option<(String, String)> {
    if node.kind() != "type_alias_declaration" {
        return None;
    }
    if let (Some(name), Some(value)) = (
        node.child_by_field_name("name")
            .and_then(|n| n.utf8_text(src).ok()),
        node.child_by_field_name("value")
            .and_then(|n| n.utf8_text(src).ok()),
    ) {
        return Some((name.trim().to_string(), value.trim().to_string()));
    }
    let text = node.utf8_text(src).ok()?.trim();
    let rest = text.strip_prefix("type ")?;
    let (name, target) = rest.split_once('=')?;
    Some((
        name.trim().to_string(),
        target.trim().trim_end_matches(';').trim().to_string(),
    ))
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
