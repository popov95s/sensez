//! Language-owned phrases and literals that smell detectors consult.

#[derive(Debug)]
pub struct SmellWording {
    /// Characters besides alphanumerics and `_` that identifiers may contain.
    pub identifier_extra_chars: &'static [char],
    /// Named structure replacing loose maps (`loose_typing`, `data_clump`).
    pub structure: &'static str,
    /// Named structure replacing positional tuples (`tuple_packing`).
    pub grouped_value: &'static str,
    /// Closed set of string categories (`literal_membership`).
    pub category: &'static str,
    /// Replacement for boolean flags (`boolean_blindness`).
    pub options: &'static str,
    /// Advice for a param typed as the safe top type (`loose_typing`).
    pub safe_top_advice: &'static str,
    /// Idiomatic string fallback syntax (`magic_string_default`).
    pub fallback_example: &'static str,
    /// Advice for a string fallback (`magic_string_default`).
    pub fallback_advice: &'static str,
    /// Placeholder words treated as missing-value sentinels (`magic_string_default`).
    pub sentinel_words: &'static [&'static str],
    /// Error handler clause name (`swallowed_error_sentinel`).
    pub handler_clause: &'static str,
    /// Advice for a handler that swallows its error (`swallowed_error_sentinel`).
    pub swallowed_error_advice: &'static str,
    /// Function names a framework calls by spelling, so copies are not helpers (`sibling_helper_redefinition`).
    pub framework_names: &'static [&'static str],
}

/// Missing-value placeholders shared by languages without `null`/`undefined` literals.
pub(crate) const SENTINEL_WORDS: &[&str] =
    &["n/a", "na", "none", "unknown", "tbd", "-", "--", "?", "??"];

/// String-fallback advice shared by languages with an optional type.
pub(crate) const FALLBACK_ADVICE: &str =
    "prefer a tighter contract with an optional string value or a dedicated default";
