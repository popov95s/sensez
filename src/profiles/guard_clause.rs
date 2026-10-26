//! Top-level guard-clause returns (`if (!x) return;` directly in a function body).

use crate::profiles::lineage::Lineage;
use tree_sitter::Node;

/// Per-language `if` and block shapes around a guard return.
pub(crate) struct GuardClauseSyntax {
    pub(crate) if_kind: &'static str,
    pub(crate) block_kinds: &'static [&'static str],
    /// Statement nodes that wrap an expression (`expression_statement` in Rust).
    pub(crate) wrapper_kinds: &'static [&'static str],
    pub(crate) is_function: fn(&str) -> bool,
}

/// `ret` is the only statement of an else-less `if` that sits directly in a function body.
pub(crate) fn is_guard_return(lineage: Lineage, syntax: &GuardClauseSyntax) -> bool {
    let arm = sole_statement_arm(lineage, unwrap(lineage, lineage.node(), syntax), syntax);
    let Some(branch) = lineage.parent(arm).filter(|p| p.kind() == syntax.if_kind) else {
        return false;
    };
    let is_consequence = branch
        .child_by_field_name("consequence")
        .is_some_and(|consequence| consequence.id() == arm.id());
    is_consequence
        && branch.child_by_field_name("alternative").is_none()
        && lineage
            .parent(unwrap(lineage, branch, syntax))
            .filter(|block| syntax.block_kinds.contains(&block.kind()))
            .and_then(|block| lineage.parent(block))
            .is_some_and(|func| (syntax.is_function)(func.kind()))
}

fn unwrap<'a>(lineage: Lineage<'_, 'a>, node: Node<'a>, syntax: &GuardClauseSyntax) -> Node<'a> {
    lineage
        .parent(node)
        .filter(|parent| syntax.wrapper_kinds.contains(&parent.kind()))
        .unwrap_or(node)
}

fn sole_statement_arm<'a>(
    lineage: Lineage<'_, 'a>,
    statement: Node<'a>,
    syntax: &GuardClauseSyntax,
) -> Node<'a> {
    lineage
        .parent(statement)
        .filter(|block| syntax.block_kinds.contains(&block.kind()) && named_statements(*block) == 1)
        .unwrap_or(statement)
}

fn named_statements(block: Node) -> usize {
    let mut cursor = block.walk();
    block
        .named_children(&mut cursor)
        .filter(|child| !child.kind().contains("comment"))
        .count()
}
