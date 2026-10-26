//! Stable identity of a supported source language.

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
}

impl Language {
    /// Every supported language, in declaration order.
    pub const ALL: [Language; 4] = [
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
        Language::Rust,
    ];

    /// Lower-case configuration key (`[smells.python]`).
    pub const fn key(self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Rust => "rust",
        }
    }
}
