//! Per-language smell configuration: built-in defaults differ per language, and
//! `[smells]` base keys + `[smells.<lang>]` tables overlay on top.

use super::SmellConfig;
use crate::config::smells::Strictness;
use crate::report::{ActionLevel, SmellKind};
use crate::spine::ir::Language;

/// The built-in defaults differ: TS enables `large_class` and disables the
/// ESLint/SonarJS-owned always-on smells; Python keeps the bare baseline.
#[test]
fn defaults_differ_per_language() {
    let cfg = SmellConfig::default();
    let py = cfg.for_language(Language::Python);
    let ts = cfg.for_language(Language::TypeScript);

    assert!(py.disabled.contains(&SmellKind::NestedLoop));
    assert!(!py.disabled.contains(&SmellKind::NestedTernary));
    assert!(py.disabled.contains(&SmellKind::NPlusOneCall));
    assert!(py.disabled.contains(&SmellKind::DefensiveFallback));
    assert!(py.disabled.contains(&SmellKind::RedundantValidation));
    assert!(
        !py.disabled.contains(&SmellKind::RedundantWrapperChain),
        "wrapper chains are checked by default"
    );
    assert_eq!(py.max_wrapper_depth, 0);
    assert!(!py.large_class, "python large_class deferred to Ruff");

    assert!(ts.large_class, "TS enables large_class (no native rule)");
    assert!(ts.disabled.contains(&SmellKind::DeepNesting));
    assert!(ts.disabled.contains(&SmellKind::HighCognitiveComplexity));
    assert!(ts.disabled.contains(&SmellKind::NestedLoop));
    assert!(ts.disabled.contains(&SmellKind::NPlusOneCall));
    assert!(ts.disabled.contains(&SmellKind::DefensiveFallback));
    assert!(ts.disabled.contains(&SmellKind::RedundantValidation));
    // JavaScript shares the TS default.
    assert!(cfg
        .for_language(Language::JavaScript)
        .disabled
        .contains(&SmellKind::DeepNesting));
    assert!(!cfg
        .for_language(Language::JavaScript)
        .disabled
        .contains(&SmellKind::NestedTernary));
    assert!(!cfg
        .for_language(Language::Rust)
        .disabled
        .contains(&SmellKind::NestedTernary));
}

/// A `[smells.<lang>]` table overrides only the keys it names; the rest keep the
/// language default. A base `[smells]` knob applies to every language.
#[test]
fn overlays_merge_onto_language_default() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            enabled = true
            loose_typing = false

            [typescript]
            max_cyclomatic = 12
            disabled = ["feature_envy"]
        "#,
    )
    .unwrap();

    let ts = cfg.for_language(Language::TypeScript);
    // Per-language override applied.
    assert_eq!(ts.max_cyclomatic, 12);
    assert_eq!(ts.disabled, vec![SmellKind::FeatureEnvy]);
    // Unspecified TS-default knob is preserved through the merge.
    assert!(
        ts.large_class,
        "TS default large_class survives a partial table"
    );
    // Base `[smells]` knob reaches every language.
    assert!(!ts.loose_typing, "base override reaches TypeScript");
    assert!(
        !cfg.for_language(Language::Python).loose_typing,
        "base override reaches Python"
    );
}

/// `enabled` defaults to true and `exclude` is shared across languages.
#[test]
fn shared_gate_and_excludes() {
    let cfg: SmellConfig = toml::from_str("exclude = [\"**/gen/**\"]").unwrap();
    assert!(cfg.enabled);
    assert_eq!(cfg.exclude, vec!["**/gen/**".to_string()]);
}

#[test]
fn nested_rule_tables_group_knobs_and_implicitly_enable() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.split_variable]
            min_assigns = 3
            action = "info"

            [rules.long_function]
            max_lines = 80
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert!(py.split_variable, "setting min_assigns implies enabled");
    assert_eq!(py.split_variable_min_assigns, 3);
    assert_eq!(py.actions[&SmellKind::SplitVariable], ActionLevel::Info);
    assert!(py.long_function, "setting max_lines implies enabled");
    assert_eq!(py.max_function_lines, 80);
}

#[test]
fn beta_performance_rules_require_explicit_opt_in() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.nested_loop]
            enabled = true
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert!(!py.disabled.contains(&SmellKind::NestedLoop));
    assert!(py.disabled.contains(&SmellKind::NPlusOneCall));
}

#[test]
fn review_risk_rules_require_explicit_opt_in() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.redundant_validation]
            enabled = true
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert!(!py.disabled.contains(&SmellKind::RedundantValidation));
    assert!(py.disabled.contains(&SmellKind::DefensiveFallback));
}

#[test]
fn wrapper_chain_depth_is_configurable_and_enables_the_rule() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.redundant_wrapper_chain]
            max_depth = 1
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert_eq!(py.max_wrapper_depth, 1);
    assert!(!py.disabled.contains(&SmellKind::RedundantWrapperChain));
}

#[test]
fn loose_typing_strictness_rule_knob_parses() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.loose_typing]
            strictness = "high"
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert!(py.loose_typing, "setting strictness implies enabled");
    assert_eq!(py.loose_typing_strictness, Strictness::High);
}

#[test]
fn narrating_code_rule_knobs_parse() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.narrating_code]
            min_comment_lines = 8
            max_comment_ratio_percent = 20
            action = "warning"
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert!(py.narrating_code);
    assert_eq!(py.min_comment_lines, 8);
    assert_eq!(py.max_comment_ratio_percent, 20);
    assert_eq!(py.actions[&SmellKind::NarratingCode], ActionLevel::Warning);
}

#[test]
fn ignore_roots_must_be_a_string_list() {
    let err = toml::from_str::<SmellConfig>(
        r#"
            [rules.message_chain]
            ignore_roots = "api"
        "#,
    )
    .unwrap_err()
    .to_string();

    assert!(
        err.contains("ignore_roots must be a list of strings"),
        "{err}"
    );
}

#[test]
fn opt_in_rule_knobs_parse() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.heavy_nested_function]
            skip_capturing = true

            [rules.tuple_packing]
            allow_homogeneous = false

            [rules.weak_test_oracle]
            assertion_functions = ["check*"]

            [rules.n_plus_one_call]
            callees = ["*.db.*"]
            max_constant_bound = 3
            await_in_loop = true
        "#,
    )
    .unwrap();

    let py = cfg.for_language(Language::Python);
    assert!(py.nested_function_skip_capturing);
    assert!(!py.tuple_allow_homogeneous);
    assert_eq!(py.assertion_functions, vec!["check*".to_string()]);
    assert!(!py.disabled.contains(&SmellKind::WeakTestOracle));
    assert_eq!(py.n_plus_one_callees, vec!["*.db.*".to_string()]);
    assert_eq!(py.n_plus_one_max_constant_bound, 3);
    assert!(py.n_plus_one_await_in_loop);
    assert!(!py.disabled.contains(&SmellKind::NPlusOneCall));
}

#[test]
fn magic_number_and_return_knobs_parse() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.magic_numbers]
            ignore = [7, 1.5]
            ignore_array_indexes = false
            ignore_default_values = false
            allow_http_status = false

            [rules.too_many_returns]
            count_guard_clauses = true
        "#,
    )
    .unwrap();

    let ts = cfg.for_language(Language::TypeScript);
    assert!(ts.magic_numbers);
    assert_eq!(
        ts.magic_number_ignore,
        vec!["7".to_string(), "1.5".to_string()]
    );
    assert!(!ts.magic_number_ignore_array_indexes);
    assert!(!ts.magic_number_ignore_default_values);
    assert!(!ts.magic_number_allow_http_status);
    assert!(ts.too_many_returns);
    assert!(ts.count_guard_clauses);

    let err = toml::from_str::<SmellConfig>("[rules.magic_numbers]\nignore = [\"7\"]\n")
        .unwrap_err()
        .to_string();
    assert!(err.contains("ignore must be a list of numbers"), "{err}");
}

#[test]
fn assertion_functions_must_be_a_string_list() {
    for value in ["\"expect*\"", "[1]", "[\"a{\"]"] {
        let err = toml::from_str::<SmellConfig>(&format!(
            "[rules.weak_test_oracle]\nassertion_functions = {value}\n"
        ))
        .unwrap_err()
        .to_string();
        assert!(err.contains("assertion_functions"), "{value}: {err}");
    }
}

#[test]
fn n_plus_one_callees_must_be_valid_globs() {
    let err = toml::from_str::<SmellConfig>("[rules.n_plus_one_call]\ncallees = [\"a{\"]\n")
        .unwrap_err()
        .to_string();
    assert!(err.contains("callees"), "{err}");
}

#[test]
fn language_rule_tables_override_base_rules() {
    let cfg: SmellConfig = toml::from_str(
        r#"
            [rules.split_variable]
            min_assigns = 3

            [typescript.rules.split_variable]
            enabled = false
        "#,
    )
    .unwrap();

    assert!(cfg.for_language(Language::Python).split_variable);
    let ts = cfg.for_language(Language::TypeScript);
    assert!(!ts.split_variable);
    assert!(ts.disabled.contains(&SmellKind::SplitVariable));
}

#[test]
fn known_keys_with_wrong_types_fail_loudly() {
    let err = toml::from_str::<SmellConfig>(
        r#"
            [python]
            max_cyclomatic = "twelve"
        "#,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("invalid [smells.python]"), "{err}");
    assert!(err.contains("max_cyclomatic"), "{err}");
}

#[test]
fn loose_typing_strictness_rejects_bad_values() {
    let err = toml::from_str::<SmellConfig>(
        r#"
            [rules.loose_typing]
            strictness = "maximum"
        "#,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("low, medium, high"), "{err}");
}
