//! Receiver and name of a method call.

use tree_sitter::Node;

/// Method name and receiver of a `receiver.method(..)` call.
pub(super) fn method_call<'a>(call: Node<'a>, src: &'a [u8]) -> Option<(&'a str, Node<'a>)> {
    let function = call
        .child_by_field_name("function")
        .filter(|f| f.kind() == "field_expression")?;
    let method = function.child_by_field_name("field")?.utf8_text(src).ok()?;
    Some((method, function.child_by_field_name("value")?))
}
