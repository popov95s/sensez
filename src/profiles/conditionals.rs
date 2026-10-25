//! Shared conditional-shape helpers for language unit walkers.
//!
//! Each grammar names `if`, then-body, and else/alternative fields differently,
//! but the smell semantics are language-neutral: an outer `if` with no
//! alternative whose entire then-body is one inner `if` with no alternative.

use crate::spine::ir::NestedTernary;
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

pub(crate) struct TernaryShape<'a> {
    pub(crate) conditional_kind: &'a str,
    pub(crate) transparent_kinds: &'a [&'a str],
    pub(crate) function_kinds: &'a [&'a str],
    /// Markup or literal nodes that end the lexical nesting walk: JSX elements, object literals.
    pub(crate) boundary_kinds: &'a [&'a str],
}

/// Chain position of `node` through transparent parents, else a lexical-only record.
pub(crate) fn ternary_nesting(node: Node<'_>, shape: &TernaryShape<'_>) -> Option<NestedTernary> {
    let mut depth = 0;
    let mut root = node;
    let mut current = node;
    while let Some(parent) = current.parent() {
        if parent.kind() == shape.conditional_kind {
            depth += 1;
            root = parent;
        } else if !shape.transparent_kinds.contains(&parent.kind()) {
            break;
        }
        current = parent;
    }
    let nested = depth > 0 || is_nested_conditional(node, shape);
    nested.then(|| NestedTernary {
        line: node.start_position().row + 1,
        root_line: root.start_position().row + 1,
        depth,
    })
}

fn is_nested_conditional(node: Node<'_>, shape: &TernaryShape<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(current) = parent {
        if current.kind() == shape.conditional_kind {
            return true;
        }
        if shape.function_kinds.contains(&current.kind())
            || shape.boundary_kinds.contains(&current.kind())
        {
            return false;
        }
        parent = current.parent();
    }
    false
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
