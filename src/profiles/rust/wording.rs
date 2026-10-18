use crate::profiles::wording::SmellWording;

pub(crate) const WORDING: SmellWording = SmellWording {
    identifier_extra_chars: &[],
    structure: "a struct",
    grouped_value: "a struct",
    category: "an enum",
    options: "an enum",
    safe_top_advice: "parse it into a concrete type at the boundary",
    fallback_example: r#"`unwrap_or("")`"#,
    fallback_advice:
        "prefer a tighter contract with an optional string value or a dedicated default",
    sentinel_words: &["n/a", "na", "none", "unknown", "tbd", "-", "--", "?", "??"],
};
