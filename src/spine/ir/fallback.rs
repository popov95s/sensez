//! String-literal fallbacks and the place their value flows into.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackSink {
    Rendered,
    StringOp,
    Stored,
    Compared,
    Returned,
    Other,
    /// The language profile does not classify sinks.
    #[default]
    Unclassified,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StringFallback {
    pub line: usize,
    pub literal: String,
    pub sink: FallbackSink,
}
