/// Result of classifying a symbol's decorator set for dead-code liveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoratorClass {
    /// No decorators (or the language has none).
    None,
    /// Structural/stdlib decorators only — no effect on liveness.
    Neutral,
    /// Framework-registration decorator present — treat the symbol as live.
    Registration,
    /// A bare unrecognized decorator — uncertain, downgrade confidence.
    Unknown,
}

impl DecoratorClass {
    pub(crate) fn is_registration(self) -> bool {
        matches!(self, Self::Registration)
    }

    pub(crate) fn is_unknown(self) -> bool {
        matches!(self, Self::Unknown)
    }
}
