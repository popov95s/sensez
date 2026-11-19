use super::normalize_jsonc;

#[test]
fn preserves_escaped_quotes_and_comment_markers_in_strings() {
    let input = r#"{ "label": "escaped \" // text", "trailing": 1, }"#;
    let normalized = normalize_jsonc(input);
    let value: serde_json::Value = serde_json::from_str(&normalized).unwrap();
    assert_eq!(value["label"], "escaped \" // text");
    assert_eq!(value["trailing"], 1);
}

#[test]
fn strips_comments_and_trailing_commas_into_parseable_json() {
    let input = "{ /* c */ \"a\": [1, 2,], // line\n \"b\": { \"c\": 1, }, }";
    let normalized = normalize_jsonc(input);
    let value: serde_json::Value = serde_json::from_str(&normalized)
        .unwrap_or_else(|err| panic!("normalized output must be valid JSON: {err}\n{normalized}"));
    assert_eq!(value["a"], serde_json::json!([1, 2]));
    assert_eq!(value["b"]["c"], 1);
    assert!(!normalized.contains("//"), "line comments must be stripped");
}

#[test]
fn drops_trailing_comma_followed_by_a_comment() {
    let input = "{ \"a\": { \"b\": 1 },  // note\n /* more */ }";
    let normalized = normalize_jsonc(input);
    let value: serde_json::Value =
        serde_json::from_str(&normalized).unwrap_or_else(|err| panic!("{err}\n{normalized}"));
    assert_eq!(value["a"]["b"], 1);
}
