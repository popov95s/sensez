//! JS/TS identifier rule for duplication lexemes (core in [`crate::profiles::lexeme`]).
//!
//! Member/property names are API surface — kept verbatim. Only function-bound
//! locals collapse.

use crate::profiles::lexeme as shared;
use crate::spine::ir::tokens::StructuralToken;
use std::collections::HashSet;
use tree_sitter::Node;

/// Lexeme code for an emitted token. `fn_bounds` is the stack of enclosing
/// functions' bound-name sets (innermost last).
pub fn code(node: Node, tok: StructuralToken, src: &[u8], fn_bounds: &[HashSet<String>]) -> u64 {
    // property_identifier / shorthand property keys are API surface — always kept.
    shared::code_with_api_surface(node, tok, src, fn_bounds, |node| {
        node.kind() != "identifier" || is_member_property(node)
    })
}

fn is_member_property(node: Node) -> bool {
    node.parent().is_some_and(|p| {
        p.kind() == "member_expression"
            && p.child_by_field_name("property").map(|a| a.id()) == Some(node.id())
    })
}
