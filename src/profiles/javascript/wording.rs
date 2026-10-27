use crate::profiles::wording::SmellWording;

pub(crate) const WORDING: SmellWording = SmellWording {
    identifier_extra_chars: &['$'],
    structure: "a typed object or interface",
    grouped_value: "an object type or a labeled tuple (`[r: number, g: number]`)",
    category: "a string-literal union or `as const` object",
    options: "an options object or a string-literal union",
    safe_top_advice: "narrow it with a type guard or schema parse at the boundary",
    fallback_example: r#"`?? ""` / `|| ""`"#,
    fallback_advice: "pass `undefined`/`null` through or reject the missing value",
    sentinel_words: &[
        "n/a",
        "na",
        "none",
        "null",
        "undefined",
        "unknown",
        "tbd",
        "-",
        "--",
        "?",
        "??",
    ],
    handler_clause: "catch",
    swallowed_error_advice:
        "rethrow it or return a typed failure (`{ ok: false, error }`) the caller must handle",
    framework_names: &[
        "loader", "action", "handler", "main", "default", "setup", "render",
    ],
    double_api_prefixes: &["mock"],
    test_file_prefix: "",
    test_file_suffix: "",
};
