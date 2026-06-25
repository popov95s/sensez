//! Stable identity of a supported source language.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
}
