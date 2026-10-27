use super::SmellConfig;

const OVERRIDES: &str = r#"
max_params = 7
exclude = ["gen/**"]
[python]
max_nesting = 2
[typescript.rules.long_function]
max_lines = 40
[rust]
large_class = true
"#;

fn configured() -> SmellConfig {
    toml::from_str(OVERRIDES).unwrap()
}

#[test]
fn serialized_config_lists_languages_in_declaration_order() {
    let text = toml::to_string(&configured()).unwrap();
    let positions: Vec<usize> = ["[python]", "[javascript]", "[typescript]", "[rust]"]
        .iter()
        .map(|header| text.find(header).unwrap())
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{text}");
}

#[test]
fn serialized_config_round_trips() {
    let text = toml::to_string(&configured()).unwrap();
    let reparsed: SmellConfig = toml::from_str(&text).unwrap();
    assert_eq!(toml::to_string(&reparsed).unwrap(), text);
    assert_eq!(
        serde_json::to_string(&reparsed).unwrap(),
        serde_json::to_string(&configured()).unwrap()
    );
}
