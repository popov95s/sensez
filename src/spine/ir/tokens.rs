//! Structural token vocabulary.

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum StructuralToken {
    FunctionDef,
    ClassDef,
    IfStatement,
    ForStatement,
    WhileStatement,
    TryStatement,
    WithStatement,
    Assign,
    BinaryOp,
    Call,
    Return,
    GenericIdentifier,
    GenericLiteral,
}

use super::FileId;

/// Source location of the token at the same index; rows are 1-indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenSpan {
    pub file_id: FileId,
    pub start_row: u32,
    pub end_row: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}
