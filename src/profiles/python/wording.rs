use crate::profiles::wording::SmellWording;

pub(crate) const WORDING: SmellWording = SmellWording {
    structure: "a dataclass or model",
    grouped_value: "a NamedTuple or dataclass",
    category: "an Enum",
    options: "an Enum",
    fallback_example: r#"`or ""`"#,
    fallback_advice:
        "prefer a tighter contract with an optional string value or a dedicated default",
    sentinel_words: &["n/a", "na", "none", "unknown", "tbd", "-", "--", "?", "??"],
};
