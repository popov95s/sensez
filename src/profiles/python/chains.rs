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
