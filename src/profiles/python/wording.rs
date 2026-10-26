use crate::profiles::wording::{self, SmellWording};

pub(crate) const WORDING: SmellWording = SmellWording {
    identifier_extra_chars: &[],
    structure: "a dataclass or model",
    grouped_value: "a NamedTuple or dataclass",
    category: "an Enum",
    options: "an Enum",
    safe_top_advice: "narrow it with isinstance checks or a model parse at the boundary",
    fallback_example: r#"`or ""`"#,
    fallback_advice: wording::FALLBACK_ADVICE,
    sentinel_words: wording::SENTINEL_WORDS,
};
