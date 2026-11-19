//! Rust identifier rule for duplication lexemes (core in [`crate::profiles::lexeme`]).

use crate::profiles::lexeme as shared;
use crate::profiles::lexeme::BoundNames;
use crate::profiles::node_names::NodeNames;
use crate::spine::ir::tokens::StructuralToken;
use tree_sitter::Node;

/// Lexeme code for an emitted token.
pub fn code(node: Node, tok: StructuralToken, src: &[u8], fn_bounds: &[BoundNames]) -> u64 {
    // Field/method names, types, and `self` are API surface — always kept.
    shared::code_with_api_surface(node, tok, src, fn_bounds, |node| {
        node.kind_name() != "identifier"
    })
}
