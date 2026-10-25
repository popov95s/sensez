//! Visibility of functions and class members.

use crate::profiles::lineage::Lineage;
use crate::spine::ir::{CallbackPosition, Visibility};
use tree_sitter::Node;

pub(super) fn method_visibility(func: Node, src: &[u8]) -> Visibility {
    private_when(is_private_method(func, src))
}

pub(super) fn function_visibility(func: Lineage) -> Visibility {
    private_when(
        func.parent(declaration(func))
            .is_none_or(|p| p.kind() != "export_statement"),
    )
}

/// The statement declaring `func`: itself, or the `const` binding an arrow or function expression.
fn declaration<'tree>(func: Lineage<'_, 'tree>) -> Node<'tree> {
    func.parent(func.node())
        .filter(|parent| parent.kind() == "variable_declarator")
        .and_then(|declarator| func.parent(declarator))
        .unwrap_or(func.node())
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

/// Callee name when `func` is passed directly as a call argument.
pub(super) fn callback_position(func: Lineage) -> CallbackPosition {
    let is_argument = func
        .parent(func.node())
        .filter(|p| p.kind() == "arguments")
        .and_then(|args| func.parent(args))
        .is_some_and(|call| call.kind() == "call_expression");
    match is_argument {
        true => CallbackPosition::Argument,
        false => CallbackPosition::Standalone,
    }
}
