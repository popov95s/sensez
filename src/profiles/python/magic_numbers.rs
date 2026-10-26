//! Python literal contexts for the shared magic-number rules.

use crate::profiles::lineage::Lineage;
use crate::profiles::magic_number::MagicSyntax;

pub(super) const SYNTAX: MagicSyntax = MagicSyntax {
    number_kinds: &["integer", "float"],
    product_kind: "binary_operator",
    arguments_kind: "argument_list",
    call_kind: "call",
    exempt_callees: &["sleep", "round", "zfill", "ljust", "rjust"],
    subscript_kind: "subscript",
    index_field: "subscript",
    status_names: &["status", "status_code"],
    status_parents: &[
        "comparison_operator",
        "assignment",
        "pair",
        "keyword_argument",
    ],
    is_named_constant,
    default_value: |param| param.child_by_field_name("value"),
};

/// `TTL_SECONDS = 300`.
fn is_named_constant(lineage: Lineage, src: &[u8]) -> bool {
    let node = lineage.node();
    lineage
        .parent(node)
        .filter(|assignment| assignment.kind() == "assignment")
        .filter(|assignment| {
            assignment
                .child_by_field_name("right")
                .is_some_and(|right| right.id() == node.id())
        })
        .and_then(|assignment| assignment.child_by_field_name("left"))
        .and_then(|left| left.utf8_text(src).ok())
        .is_some_and(|name| {
            name.chars()
                .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
        })
}
