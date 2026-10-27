//! JS/TS literal contexts for the shared magic-number rules.

use crate::profiles::lineage::Lineage;
use crate::profiles::magic_number::MagicSyntax;
use crate::profiles::node_names::NodeNames;
use tree_sitter::Node;

pub(super) const SYNTAX: MagicSyntax = MagicSyntax {
    number_kinds: &["number"],
    product_kind: "binary_expression",
    arguments_kind: "arguments",
    call_kind: "call_expression",
    exempt_callees: &[
        "setTimeout",
        "setInterval",
        "sleep",
        "delay",
        "toFixed",
        "toPrecision",
        "padStart",
        "padEnd",
        "toString",
    ],
    subscript_kind: "subscript_expression",
    index_field: "index",
    status_names: &["status", "statusCode", "sendStatus"],
    status_parents: &[
        "binary_expression",
        "assignment_expression",
        "pair",
        "variable_declarator",
    ],
    is_named_constant,
    default_value,
};

fn default_value(param: Node) -> Option<Node> {
    match param.kind_name() {
        "assignment_pattern" => param.field("right"),
        _ => param.field("value"),
    }
}

/// `const TTL_MS = 5000`.
fn is_named_constant(lineage: Lineage, src: &[u8]) -> bool {
    let node = lineage.node();
    lineage
        .parent(node)
        .filter(|declarator| declarator.kind_name() == "variable_declarator")
        .filter(|declarator| {
            declarator
                .field("value")
                .is_some_and(|value| value.id() == node.id())
        })
        .and_then(|declarator| lineage.parent(declarator))
        .and_then(|declaration| declaration.field("kind"))
        .is_some_and(|kind| kind.utf8_text(src) == Ok("const"))
}
