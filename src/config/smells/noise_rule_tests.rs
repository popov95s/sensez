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

            [rules.swallowed_error_sentinel]
            strictness = "low"
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert_eq!(py.narrating_min_function_lines, 20);
    assert!(py.repeated_iteration_include_tests);
    assert_eq!(py.magic_string_default_strictness, Strictness::Low);
    assert_eq!(py.nested_ternary_strictness, Strictness::High);
    assert_eq!(py.swallowed_error_strictness, Strictness::Low);
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

#[test]
fn precision_rule_knobs_parse() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.message_chain]
            ignore_roots = ["api"]

            [rules.literal_membership]
            skip_external_vocab = false

            [rules.god_module]
            min_fan_in = 2
            min_fan_out = 3
            min_lines = 40
            min_units = 4
        "#,
    )
    .unwrap();

    let ts = cfg.for_language(Language::TypeScript);
    assert_eq!(ts.message_chain_ignore_roots, vec!["api".to_string()]);
    assert!(!ts.literal_membership_skip_external_vocab);
    assert_eq!(ts.god_module_min_fan_in, 2);
    assert_eq!(ts.god_module_min_fan_out, 3);
    assert_eq!(ts.god_module_min_lines, 40);
    assert_eq!(ts.god_module_min_units, 4);

    let defaults = SmellConfig::default();
    let py = defaults.for_language(Language::Python);
    assert!(py.message_chain_ignore_roots.is_empty());
    assert!(py.literal_membership_skip_external_vocab);
    assert_eq!(
        (py.god_module_min_fan_in, py.god_module_min_fan_out),
        (5, 10)
    );
    assert_eq!(
        (py.god_module_min_lines, py.god_module_min_units),
        (150, 12)
    );
}

#[test]
fn dependency_reimplementation_providers_parse() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.dependency_reimplementation]
            providers = ["@acme/kit"]
        "#,
    )
    .unwrap();

    let ts = cfg.for_language(Language::TypeScript);
    assert_eq!(
        ts.dependency_reimplementation_providers,
        vec!["@acme/kit".to_string()]
    );
}
