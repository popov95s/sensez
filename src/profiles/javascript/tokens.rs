//! tree-sitter-javascript node-kind → [`StructuralToken`] mapping.
//!
//! Targets the same shared 13-token vocabulary as every other language (never a
//! per-language enum), so JS-specific constructs (JSX, `switch`, ternaries)
//! collapse onto existing tokens or emit nothing. Identifiers and literals
//! genericize; unrecognized kinds return `None`.

use crate::spine::ir::tokens::StructuralToken;

/// Map a node kind to its structural token, or `None` if not significant.
pub fn map_kind(kind: &str) -> Option<StructuralToken> {
    use StructuralToken::*;
    Some(match kind {
        "function_declaration"
        | "function_expression"
        | "function"
        | "arrow_function"
        | "generator_function"
        | "generator_function_declaration"
        | "method_definition" => FunctionDef,
        "class_declaration" | "class" | "abstract_class_declaration" => ClassDef,
        "if_statement" => IfStatement,
        "for_statement" | "for_in_statement" => ForStatement,
        "while_statement" | "do_statement" => WhileStatement,
        "try_statement" => TryStatement,
        "with_statement" => WithStatement,
        "assignment_expression"
        | "augmented_assignment_expression"
        | "variable_declaration"
        | "lexical_declaration" => Assign,
        "binary_expression" => BinaryOp,
        "call_expression" => Call,
        "return_statement" => Return,
        "identifier" | "property_identifier" | "shorthand_property_identifier" => GenericIdentifier,
        "number" | "string" | "template_string" | "true" | "false" | "null" | "undefined"
        | "regex" => GenericLiteral,
        _ => return None,
    })
}

/// Leaf kinds whose children must not be traversed.
pub fn is_leaf(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "property_identifier"
            | "shorthand_property_identifier"
            | "number"
            | "string"
            | "template_string"
            | "true"
            | "false"
            | "null"
            | "undefined"
            | "regex"
    )
}

/// Kinds that open a new lexical scope (function/class — for `enclosing_scope`
/// and method detection).
pub fn is_scope(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "function_expression"
            | "function"
            | "arrow_function"
            | "generator_function"
            | "generator_function_declaration"
            | "method_definition"
            | "class_declaration"
            | "class"
            | "abstract_class_declaration"
    )
}

/// True for class-like scope openers (method detection).
pub fn is_class(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration" | "class" | "abstract_class_declaration"
    )
}
