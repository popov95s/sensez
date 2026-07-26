//! Python identifier rule for duplication lexemes (core in [`crate::profiles::lexeme`]).
//!
//! Attribute/method names and keyword-argument names are API surface, never
//! local variables — kept verbatim. Only function-bound locals collapse.

use crate::profiles::lexeme as shared;
use crate::spine::ir::tokens::StructuralToken;
use std::collections::HashSet;
use tree_sitter::Node;

/// Compute the lexeme code for an emitted token. `fn_bounds` is the stack of
/// enclosing functions' bound-name sets (innermost last).
pub fn code(node: Node, tok: StructuralToken, src: &[u8], fn_bounds: &[HashSet<String>]) -> u64 {
    shared::code_with_api_surface(node, tok, src, fn_bounds, |node| {
        is_attribute_name(node) || is_keyword_arg_name(node)
    })
}

fn is_attribute_name(node: Node) -> bool {
    node.parent().is_some_and(|p| {
        p.kind() == "attribute"
            && p.child_by_field_name("attribute").map(|a| a.id()) == Some(node.id())
    })
}

fn is_keyword_arg_name(node: Node) -> bool {
    node.parent().is_some_and(|p| {
        p.kind() == "keyword_argument"
            && p.child_by_field_name("name").map(|a| a.id()) == Some(node.id())
    })
}
