//! tree-sitter-rust node-kind → [`StructuralToken`] mapping.

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

/// Leaf kinds whose children are not traversed.
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

/// Kinds that open a new lexical scope.
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
