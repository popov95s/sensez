//! Shared lexeme-code core for refined (Type-2) duplication matching.

use crate::profiles::node_names::NodeNames;
use crate::spine::ir::tokens::StructuralToken;
use rustc_hash::FxHashSet;
use tree_sitter::Node;

pub(crate) type BoundNames = FxHashSet<u64>;

/// Code for any function-bound local variable (collapsed; renameable).
pub(crate) const LOCAL: u64 = 1;
/// Reserved low codes (LOCAL + structural kinds) never collide with hashes.
const HASH_BASE: u64 = 16;
/// Keeps hashed lexemes below the separator namespace used by `flatten`.
const HASH_SPAN: u64 = 1 << 62;

/// Lexeme code for an emitted token.
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
        BinaryOp => hash_operator(operator(node)),
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

/// Classify an identifier as stable API surface, a renameable local, or a stable free name.
fn identifier_code(
    node: Node,
    src: &[u8],
    fn_bounds: &[BoundNames],
    is_api_surface: impl FnOnce(Node) -> bool,
) -> u64 {
    let text = node.utf8_text(src).unwrap_or_default();
    let code = hash(text);
    if is_api_surface(node) {
        return code;
    }
    if fn_bounds.iter().rev().any(|names| names.contains(&code)) {
        return LOCAL;
    }
    code
}

/// Apply the shared token mapping and identifier classification in one call.
pub(crate) fn code_with_api_surface(
    node: Node,
    tok: StructuralToken,
    src: &[u8],
    fn_bounds: &[BoundNames],
    is_api_surface: impl FnOnce(Node) -> bool,
) -> u64 {
    code(node, tok, src, || {
        identifier_code(node, src, fn_bounds, is_api_surface)
    })
}

/// The operator token of a binary or comparison node.
fn operator(node: Node) -> &'static str {
    if let Some(op) = node.field("operator") {
        return op.kind_name();
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            return child.kind_name();
        }
    }
    "op"
}

fn hash_operator(operator: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    0x6f70_u16.hash(&mut hasher);
    operator.hash(&mut hasher);
    HASH_BASE + (hasher.finish() % (HASH_SPAN - HASH_BASE))
}

/// Rehash a code for a type-declaration position so it rarely equals the value code.
pub(crate) fn type_position(code: u64) -> u64 {
    rehash(code, 0x9e37_79b9_7f4a_7c15)
}

/// Rehash a code for a shorthand property key so it differs from the same name read as a member.
pub(crate) fn shorthand_position(code: u64) -> u64 {
    rehash(code, 0xc2b2_ae3d_27d4_eb4f)
}

fn rehash(code: u64, salt: u64) -> u64 {
    HASH_BASE + ((code ^ salt) % (HASH_SPAN - HASH_BASE))
}

pub(crate) fn hash(text: &str) -> u64 {
    // Non-cryptographic fxhash: codes never face adversarial input and are compared within one family only.
    use std::hash::BuildHasher;
    HASH_BASE + (rustc_hash::FxBuildHasher.hash_one(text) % (HASH_SPAN - HASH_BASE))
}
