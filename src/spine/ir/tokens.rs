//! Structural token vocabulary.
//!
//! A [`StructuralToken`] stream is a *genericized* projection of the AST: all
//! identifiers collapse to [`StructuralToken::GenericIdentifier`] and all
//! literals to [`StructuralToken::GenericLiteral`]. Two functions that share a
//! control-flow shape but differ only in names/values therefore produce
//! byte-identical token vectors — the basis of structural clone detection.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
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

/// Source location for the token at the same index in the token vector.
///
/// Rows are 1-indexed (tree-sitter reports 0-indexed). Lets the duplication
/// pillar map master-buffer offsets back to concrete file + line ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct TokenSpan {
    pub file_id: u32,
    pub start_row: usize,
    pub end_row: usize,
}
