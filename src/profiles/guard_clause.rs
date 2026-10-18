//! Top-level guard-clause returns (`if (!x) return;` directly in a function body).

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
pub(crate) fn is_guard_return(ret: Node, syntax: &GuardClauseSyntax) -> bool {
    let arm = sole_statement_arm(unwrap(ret, syntax), syntax);
    let Some(branch) = arm.parent().filter(|p| p.kind() == syntax.if_kind) else {
        return false;
    };
    let is_consequence = branch
        .child_by_field_name("consequence")
        .is_some_and(|consequence| consequence.id() == arm.id());
    is_consequence
        && branch.child_by_field_name("alternative").is_none()
        && unwrap(branch, syntax)
            .parent()
            .filter(|block| syntax.block_kinds.contains(&block.kind()))
            .and_then(|block| block.parent())
            .is_some_and(|func| (syntax.is_function)(func.kind()))
}

fn unwrap<'a>(node: Node<'a>, syntax: &GuardClauseSyntax) -> Node<'a> {
    node.parent()
        .filter(|parent| syntax.wrapper_kinds.contains(&parent.kind()))
        .unwrap_or(node)
}

fn sole_statement_arm<'a>(statement: Node<'a>, syntax: &GuardClauseSyntax) -> Node<'a> {
    statement
        .parent()
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
