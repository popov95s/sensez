//! Shared conditional-shape helpers for language unit walkers.
//!
//! Each grammar names `if`, then-body, and else/alternative fields differently,
//! but the smell semantics are language-neutral: an outer `if` with no
//! alternative whose entire then-body is one inner `if` with no alternative.

use tree_sitter::Node;

pub(crate) struct IfShape<'a> {
    pub(crate) if_kind: &'a str,
    pub(crate) then_field: &'a str,
    pub(crate) else_field: &'a str,
    pub(crate) block_kinds: &'a [&'a str],
    pub(crate) ignored_kinds: &'a [&'a str],
}

pub(crate) fn is_collapsible_nested_if(node: Node<'_>, shape: &IfShape<'_>) -> bool {
    if node.kind() != shape.if_kind || has_else(node, shape) {
        return false;
    }
    let Some(inner) = node
        .child_by_field_name(shape.then_field)
        .and_then(|then| only_statement(then, shape))
    else {
        return false;
    };
    inner.kind() == shape.if_kind && !has_else(inner, shape)
}

fn has_else(node: Node<'_>, shape: &IfShape<'_>) -> bool {
    node.child_by_field_name(shape.else_field).is_some()
}

fn only_statement<'tree>(node: Node<'tree>, shape: &IfShape<'_>) -> Option<Node<'tree>> {
    if !shape.block_kinds.contains(&node.kind()) {
        return Some(node);
    }
    let mut cursor = node.walk();
    let mut found = None;
    for child in node.named_children(&mut cursor) {
        if shape.ignored_kinds.contains(&child.kind()) {
            continue;
        }
        if found.replace(child).is_some() {
            return None;
        }
    }
    found
}
