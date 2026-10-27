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
    handler_clause: "except",
    swallowed_error_advice:
        "re-raise it (`raise ... from error`) or return a typed failure the caller must handle",
    framework_names: &["main", "handler", "setup"],
    double_api_prefixes: &["mock", "assert_"],
    test_file_prefix: "test_",
    test_file_suffix: "_test",
    markup_names: &["tag", "tag_name", "local", "local_name", "node_name"],
    write_methods: &["append", "extend", "pop", "remove", "clear"],
    private_name_prefixes: &['_'],
};
