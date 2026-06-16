//! tree-sitter-rust node-kind → [`StructuralToken`] mapping.
//!
//! Targets the shared 13-token vocabulary (never a per-language enum), so
//! Rust-specific constructs collapse onto existing tokens or emit nothing:
//! `match` is a multi-way branch (→ `IfStatement`), `loop` is an unconditional
//! `while`, `?` is structured error propagation (→ `TryStatement`), and
//! `struct`/`enum`/`trait`/`impl` are all type-shaped containers (→ `ClassDef`).
//! Unrecognized kinds return `None` (the walker descends but emits nothing).

use crate::spine::ir::tokens::StructuralToken;

/// Map a node kind to its structural token, or `None` if not significant.
pub fn map_kind(kind: &str) -> Option<StructuralToken> {
    use StructuralToken::*;
    Some(match kind {
        "function_item" | "closure_expression" => FunctionDef,
        "struct_item" | "enum_item" | "union_item" | "trait_item" | "impl_item" => ClassDef,
        "if_expression" | "match_expression" => IfStatement,
        "for_expression" => ForStatement,
        "while_expression" | "loop_expression" => WhileStatement,
        "try_expression" => TryStatement,
        "let_declaration" | "assignment_expression" | "compound_assignment_expr" => Assign,
        "binary_expression" => BinaryOp,
        "call_expression" | "macro_invocation" => Call,
        "return_expression" => Return,
        "identifier"
        | "field_identifier"
        | "type_identifier"
        | "shorthand_field_identifier"
        | "primitive_type"
        | "self" => GenericIdentifier,
        "integer_literal" | "float_literal" | "string_literal" | "raw_string_literal"
        | "char_literal" | "boolean_literal" => GenericLiteral,
        _ => return None,
    })
}

/// Leaf kinds whose children must not be traversed (prevents literal/identifier
/// sub-nodes from leaking extra tokens into the genericized stream).
pub fn is_leaf(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "field_identifier"
            | "type_identifier"
            | "shorthand_field_identifier"
            | "primitive_type"
            | "self"
            | "integer_literal"
            | "float_literal"
            | "string_literal"
            | "raw_string_literal"
            | "char_literal"
            | "boolean_literal"
    )
}

/// Kinds that open a new lexical scope (for `enclosing_scope`, method
/// detection, and keeping inline `mod tests { … }` items out of the module's
/// top-level declarations).
pub fn is_scope(kind: &str) -> bool {
    matches!(
        kind,
        "function_item" | "closure_expression" | "impl_item" | "trait_item" | "mod_item"
    )
}

/// Class-like scope openers: a `function_item` directly inside one is a method.
pub fn is_class(kind: &str) -> bool {
    matches!(kind, "impl_item" | "trait_item")
}
