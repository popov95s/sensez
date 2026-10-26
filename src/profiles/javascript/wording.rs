use crate::profiles::wording::SmellWording;

pub(crate) const WORDING: SmellWording = SmellWording {
    structure: "a typed object or interface",
    grouped_value: "an interface or named object type",
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
};
