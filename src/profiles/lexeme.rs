//! Shared lexeme-code core for refined (Type-2) duplication matching.
//!
//! A clone is a run of identical lexeme codes. The codes keep real meaning —
//! free names, attribute/method names, types, literals, operators — and
//! collapse *only* function-bound local variables to [`LOCAL`]. The mapping
//! from structural token to code is language-neutral; each language supplies
//! only its identifier rule (which names are API surface vs. renameable
//! locals) via the `identifier_code` closure.

use crate::spine::ir::tokens::StructuralToken;
use tree_sitter::Node;

/// Code for any function-bound local variable (collapsed; renameable).
pub(crate) const LOCAL: u64 = 1;
/// Reserved low codes (LOCAL + structural kinds) never collide with hashes.
const HASH_BASE: u64 = 16;
/// Keeps hashed lexemes below the separator namespace used by `flatten`.
const HASH_SPAN: u64 = 1 << 62;

/// Lexeme code for an emitted token. `identifier_code` is the per-language
/// rule for [`StructuralToken::GenericIdentifier`].
pub(crate) fn code(
    node: Node,
    tok: StructuralToken,
    src: &[u8],
    identifier_code: impl FnOnce() -> u64,
) -> u64 {
    use StructuralToken::*;
    match tok {
        GenericIdentifier => identifier_code(),
        GenericLiteral => hash(node.utf8_text(src).unwrap_or_default()),
        BinaryOp => hash(&format!("op:{}", operator(node))),
        FunctionDef => 2,
        ClassDef => 3,
        IfStatement => 4,
        ForStatement => 5,
        WhileStatement => 6,
        TryStatement => 7,
        WithStatement => 8,
        Assign => 9,
        Call => 10,
        Return => 11,
    }
}

/// The operator token of a binary/comparison node: the `operator` field where
/// the grammar names one (JS, Rust, most Python operators), else the first
/// anonymous child (Python `comparison_operator` chains).
fn operator(node: Node) -> String {
    if let Some(op) = node.child_by_field_name("operator") {
        return op.kind().to_string();
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            return child.kind().to_string();
        }
    }
    "op".to_string()
}

pub(crate) fn hash(text: &str) -> u64 {
    // Non-cryptographic: lexeme codes are an internal equality alphabet, never
    // exposed to adversarial input, so SipHash's DoS-resistance is wasted cost.
    // fxhash is a single multiply-rotate per word — ~2-3x faster in this
    // per-identifier hot loop. Folded into [HASH_BASE, HASH_SPAN) so codes stay
    // below the separator namespace and clear of the reserved structural codes.
    // Within-language only (duplication partitions by language before flatten),
    // so codes are never compared across languages and need not agree.
    use std::hash::BuildHasher;
    HASH_BASE + (rustc_hash::FxBuildHasher.hash_one(text) % (HASH_SPAN - HASH_BASE))
}
