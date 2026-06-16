//! Rust identifier rule for duplication lexemes (core in [`crate::profiles::lexeme`]).
//!
//! Field/method names, types, and `self` are API surface — kept verbatim.
//! Only function-bound locals collapse.

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
    // Field/method names, types, and `self` are API surface — always kept.
    if node.kind() != "identifier" {
        return shared::hash(text);
    }
    if fn_bounds.iter().rev().any(|s| s.contains(text)) {
        return shared::LOCAL;
    }
    shared::hash(text)
}
