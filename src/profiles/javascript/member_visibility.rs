//! Visibility of functions and class members.

use crate::profiles::lineage::Lineage;
use crate::spine::ir::Visibility;
use tree_sitter::Node;

pub(super) fn method_visibility(func: Node, src: &[u8]) -> Visibility {
    private_when(is_private_method(func, src))
}

pub(super) fn function_visibility(func: Lineage) -> Visibility {
    private_when(
        func.parent(func.node())
            .is_none_or(|p| p.kind() != "export_statement"),
    )
}

fn private_when(private: bool) -> Visibility {
    if private {
        Visibility::Private
    } else {
        Visibility::Exported
    }
}

fn is_private_method(func: Node, src: &[u8]) -> bool {
    let hash_name = func
        .child_by_field_name("name")
        .is_some_and(|n| n.kind() == "private_property_identifier");
    let mut cursor = func.walk();
    let modifier = func
        .children(&mut cursor)
        .any(|c| c.kind() == "accessibility_modifier" && c.utf8_text(src) == Ok("private"));
    hash_name || modifier
}
