//! JS/TS identifier rule for duplication lexemes (core in [`crate::profiles::lexeme`]).
//!
//! Member/property names are API surface — kept verbatim. Only function-bound
//! locals collapse.

use crate::profiles::lexeme as shared;
use crate::profiles::lexeme::BoundNames;
use crate::spine::ir::tokens::StructuralToken;
use tree_sitter::Node;

/// Lexeme code for an emitted token. `fn_bounds` is the stack of enclosing
/// functions' bound-name sets (innermost last).
pub fn code(
    node: Node,
    tok: StructuralToken,
    src: &[u8],
    fn_bounds: &[BoundNames],
    is_member_property: bool,
) -> u64 {
    // property_identifier / shorthand property keys are API surface — always kept.
    shared::code_with_api_surface(node, tok, src, fn_bounds, |node| {
        node.kind() != "identifier" || is_member_property
    })
}
