//! Error handlers that end the error path with a placeholder value or a log.

/// How a swallowing handler hands control back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerEnd {
    /// Returns an empty value: `null`, `undefined`, `None`, `false`, `[]`, `{}`, `""`, `0`.
    EmptyValue,
    /// Returns some other literal default (`"unknown"`, `-1`, `true`).
    DefaultLiteral,
    /// Only logs (or does nothing) and falls through.
    LogOnly,
}

/// What the protected `try` body does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardedBody {
    /// One statement around a JSON parse call.
    SingleParse,
    Other,
}

/// Whether the handler explains itself with a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerComment {
    Present,
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SentinelHandler {
    pub line: usize,
    pub end: HandlerEnd,
    /// Returned literal as written; empty for a log-only handler.
    pub literal: String,
    pub guarded: GuardedBody,
    pub comment: HandlerComment,
    /// Innermost loop frame enclosing the handler.
    pub frame: Option<usize>,
}
