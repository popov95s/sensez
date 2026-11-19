//! tree-sitter node-kind → [`StructuralToken`] mapping (genericization).

use crate::spine::ir::tokens::StructuralToken;

/// Map a node kind to its structural token, or `None` if not significant.
pub fn map_kind(kind: &str) -> Option<StructuralToken> {
    use StructuralToken::*;
    Some(match kind {
        "function_definition" => FunctionDef,
        "class_definition" => ClassDef,
        "if_statement" => IfStatement,
        "for_statement" => ForStatement,
        "while_statement" => WhileStatement,
        "try_statement" => TryStatement,
        "with_statement" => WithStatement,
        "assignment" | "augmented_assignment" => Assign,
        "binary_operator" | "boolean_operator" | "comparison_operator" => BinaryOp,
        "call" => Call,
        "return_statement" => Return,
        "identifier" => GenericIdentifier,
        "integer" | "float" | "string" | "concatenated_string" | "true" | "false" | "none" => {
            GenericLiteral
        }
        _ => return None,
    })
}

/// Kinds whose children must not be traversed: leaf literals/identifiers.
pub fn is_leaf(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "integer"
            | "float"
            | "string"
            | "concatenated_string"
            | "true"
            | "false"
            | "none"
    )
}

/// Import-statement kinds, extracted rather than descended into.
pub fn is_import(kind: &str) -> bool {
    matches!(kind, "import_statement" | "import_from_statement")
}

/// Kinds that open a new lexical scope and contribute an `enclosing_scope`.
pub fn is_scope(kind: &str) -> bool {
    matches!(kind, "function_definition" | "class_definition")
}
