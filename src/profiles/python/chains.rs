//! Attribute-access chains: length, root and nesting.

use tree_sitter::Node;

/// Length of the pure attribute chain `a.b.c.d` ending at this `attribute` node.
///
/// Only consecutive attribute accesses count — the chain does **not** traverse
/// through `call` results, so fluent/builder APIs (`q.filter().limit().all()`)
/// are not mistaken for Law-of-Demeter data navigation.
pub(super) fn chain_len(node: Node) -> usize {
    let base = match node.child_by_field_name("object") {
        Some(obj) if obj.kind() == "attribute" => chain_len(obj),
        _ => 0,
    };
    base + 1
}

/// True when `node` is the object of an enclosing attribute access, so only the
/// outermost attribute of `a.b.c` records a chain.
pub(super) fn is_chain_inner(node: Node) -> bool {
    node.parent().is_some_and(|p| {
        p.kind() == "attribute"
            && p.child_by_field_name("object").map(|o| o.id()) == Some(node.id())
    })
}

pub(super) fn chain_root(node: Node, src: &[u8]) -> Option<String> {
    let obj = node.child_by_field_name("object")?;
    match obj.kind() {
        "attribute" => chain_root(obj, src),
        "identifier" => obj.utf8_text(src).ok().map(str::to_string),
        _ => None,
    }
}
