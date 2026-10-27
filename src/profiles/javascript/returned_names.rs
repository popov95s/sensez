//! Names a `return` statement hands back.

use crate::profiles::node_names::NodeNames;
use tree_sitter::Node;

/// `return x` → `x`; `return { a, b: c }` → `a`, `c`.
pub(super) fn returned_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(value) = node.named_child(0) else {
        return Vec::new();
    };
    let names: Vec<Node> = match value.kind_name() {
        "identifier" => vec![value],
        "object" => value
            .named_children(&mut value.walk())
            .filter_map(|member| match member.kind_name() {
                "shorthand_property_identifier" => Some(member),
                "pair" => member
                    .field("value")
                    .filter(|v| v.kind_name() == "identifier"),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    names
        .into_iter()
        .filter_map(|name| name.utf8_text(src).ok().map(str::to_string))
        .collect()
}
