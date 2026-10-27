//! Type-checker silencers written inside a function body.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeEscapeKind {
    /// A value cast to the top escape type (`as any`, `cast(Any, x)`).
    AnyCast,
    /// A suppression that silences every error on the next line with no reason.
    BlanketSuppression,
    /// A cast laundered through the safe top type (`as unknown as T`).
    DoubleCast,
    /// A suppression with neither a reason nor an error code.
    UnexplainedSuppression,
    /// A non-null assertion applied straight to a call result.
    NonNullCall,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TypeEscape {
    pub line: usize,
    pub kind: TypeEscapeKind,
    /// The escape as written, for the finding message.
    pub label: String,
    /// Root identifier of the cast value, when it is one.
    pub subject: Option<String>,
}
