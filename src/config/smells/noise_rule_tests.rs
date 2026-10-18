//! Knobs of the noise-reduction and precision rules.

use super::SmellConfig;
use crate::config::smells::Strictness;
use crate::spine::ir::Language;

#[test]
fn new_noise_rule_knobs_parse() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.narrating_code]
            min_function_lines = 20

            [rules.repeated_iteration]
            include_tests = true

            [rules.magic_string_default]
            strictness = "low"

            [rules.nested_ternary]
            strictness = "high"
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert_eq!(py.narrating_min_function_lines, 20);
    assert!(py.repeated_iteration_include_tests);
    assert_eq!(py.magic_string_default_strictness, Strictness::Low);
    assert_eq!(py.nested_ternary_strictness, Strictness::High);
}

#[test]
fn nested_ternary_strictness_has_no_low_level() {
    let err = toml::from_str::<SmellConfig>(
        r#"
            [rules.nested_ternary]
            strictness = "low"
        "#,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("medium, high"), "{err}");
}
