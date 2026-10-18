//! Language-owned phrases and literals that smell detectors consult.

#[derive(Debug)]
pub struct SmellWording {
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
}
