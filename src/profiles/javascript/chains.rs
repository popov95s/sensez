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
