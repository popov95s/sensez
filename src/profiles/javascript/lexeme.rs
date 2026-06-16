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
    shared::code(node, tok, src, || identifier_code(node, src, fn_bounds))
}

fn identifier_code(node: Node, src: &[u8], fn_bounds: &[HashSet<String>]) -> u64 {
    let text = node.utf8_text(src).unwrap_or_default();
    // property_identifier / shorthand property keys are API surface — always kept.
    if node.kind() != "identifier" || is_member_property(node) {
        return shared::hash(text);
    }
    if fn_bounds.iter().rev().any(|s| s.contains(text)) {
        return shared::LOCAL;
    }
    shared::hash(text)
}

fn is_member_property(node: Node) -> bool {
    node.parent().is_some_and(|p| {
        p.kind() == "member_expression"
            && p.child_by_field_name("property").map(|a| a.id()) == Some(node.id())
    })
}
