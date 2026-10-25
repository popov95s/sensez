//! String-literal fallbacks and the place their value flows into.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackSink {
    Rendered,
    StringOp,
    /// Argument to a call that is not a string operation.
    Passed,
    Stored,
    Compared,
    Returned,
    Other,
    /// The language profile does not classify sinks.
    #[default]
    Unclassified,
}

/// Where a fallback's value flows, with the callee or key it reaches when known.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FallbackSite {
    pub sink: FallbackSink,
    pub target: String,
}

impl FallbackSite {
    pub fn to(sink: FallbackSink, target: &str) -> Self {
        FallbackSite {
            sink,
            target: target.to_string(),
        }
    }
}

impl From<FallbackSink> for FallbackSite {
    fn from(sink: FallbackSink) -> Self {
        FallbackSite::to(sink, "")
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StringFallback {
    pub line: usize,
    pub literal: String,
    pub sink: FallbackSink,
    /// Callee or key the value reaches; empty when unknown.
    pub target: String,
}
