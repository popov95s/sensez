use crate::profiles::wording::{self, SmellWording};

pub(crate) const WORDING: SmellWording = SmellWording {
    identifier_extra_chars: &[],
    structure: "a struct",
    grouped_value: "a struct",
    category: "an enum",
    options: "an enum",
    safe_top_advice: "parse it into a concrete type at the boundary",
    fallback_example: r#"`unwrap_or("")`"#,
    fallback_advice: wording::FALLBACK_ADVICE,
    sentinel_words: wording::SENTINEL_WORDS,
    handler_clause: "error arm",
    swallowed_error_advice: "propagate it with `?` or return the `Err` to the caller",
    framework_names: &["main"],
    double_api_prefixes: &[],
    test_file_prefix: "",
    test_file_suffix: "",
};
