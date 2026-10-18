//! Member-access chains: length, root and nesting.

use tree_sitter::Node;

/// Length of the pure attribute chain `a.b.c.d` ending at this `member_expression`
/// — does not traverse through call results, so fluent APIs aren't mistaken for
/// data navigation (same rule as the Python walk).
pub(super) fn chain_len(node: Node) -> usize {
    let base = match node.child_by_field_name("object") {
        Some(obj) if obj.kind() == "member_expression" => chain_len(obj),
        _ => 0,
    };
    base + 1
}

/// True when `node` is the object of an enclosing member access, so only the
/// outermost member of `a.b.c` records a chain.
pub(super) fn is_chain_inner(node: Node) -> bool {
    node.parent().is_some_and(|p| {
        p.kind() == "member_expression"
            && p.child_by_field_name("object").map(|o| o.id()) == Some(node.id())
    })
}

pub(super) fn chain_root(node: Node, src: &[u8]) -> Option<String> {
    let obj = node.child_by_field_name("object")?;
    match obj.kind() {
        "member_expression" => chain_root(obj, src),
        "identifier" | "this" => obj.utf8_text(src).ok().map(str::to_string),
        _ => None,
    }
}
