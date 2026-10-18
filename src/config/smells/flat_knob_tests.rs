//! Flat `max_*` knobs and `action` entries switch their rule on.

use super::SmellConfig;
use crate::report::{ActionLevel, SmellKind};
use crate::spine::ir::Language;

#[test]
fn flat_threshold_enables_opt_in_rule() {
    let cfg: SmellConfig =
        toml::from_str("max_function_lines = 20\nmax_class_methods = 3").unwrap();

    let py = cfg.for_language(Language::Python);
    assert!(py.long_function);
    assert_eq!(py.max_function_lines, 20);
    assert!(!py.disabled.contains(&SmellKind::LongFunction));
    assert!(py.large_class, "max_class_methods enables large_class");
}

#[test]
fn flat_threshold_enables_rule_disabled_by_language_default() {
    let cfg: SmellConfig = toml::from_str("[typescript]\nmax_nesting = 3").unwrap();

    let ts = cfg.for_language(Language::TypeScript);
    assert!(!ts.disabled.contains(&SmellKind::DeepNesting));
    assert!(cfg
        .for_language(Language::JavaScript)
        .disabled
        .contains(&SmellKind::DeepNesting));
}

#[test]
fn flat_threshold_with_explicit_toggle_stays_off() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            max_function_lines = 20
            [typescript]
            long_function = false
        "#,
    )
    .unwrap();

    assert!(cfg.for_language(Language::Python).long_function);
    assert!(!cfg.for_language(Language::TypeScript).long_function);
}

#[test]
fn flat_threshold_listed_in_disabled_stays_off() {
    let cfg: SmellConfig =
        toml::from_str("[typescript]\nmax_nesting = 3\ndisabled = [\"deep_nesting\"]").unwrap();

    let ts = cfg.for_language(Language::TypeScript);
    assert!(ts.disabled.contains(&SmellKind::DeepNesting));
}

#[test]
fn rules_enabled_false_beats_flat_threshold() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            max_function_lines = 20
            [rules.long_function]
            enabled = false
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert!(!py.long_function);
    assert!(py.disabled.contains(&SmellKind::LongFunction));
}

#[test]
fn rule_action_enables_opt_in_rule() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.nested_loop]
            action = "info"

            [rules.n_plus_one_call]
            action = "info"
            enabled = false
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert!(!py.disabled.contains(&SmellKind::NestedLoop));
    assert_eq!(py.actions[&SmellKind::NestedLoop], ActionLevel::Info);
    assert!(py.disabled.contains(&SmellKind::NPlusOneCall));
}
