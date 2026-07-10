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

    assert!(py.disabled.is_empty(), "python disables nothing by default");
    assert!(!py.large_class, "python large_class deferred to Ruff");

    assert!(ts.large_class, "TS enables large_class (no native rule)");
    assert!(ts.disabled.contains(&SmellKind::DeepNesting));
    assert!(ts.disabled.contains(&SmellKind::HighCognitiveComplexity));
    // JavaScript shares the TS default.
    assert!(cfg
        .for_language(Language::JavaScript)
        .disabled
        .contains(&SmellKind::DeepNesting));
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
fn unknown_keys_fail_loudly() {
    let err = toml::from_str::<SmellConfig>(
        r#"
            typo_toggle = true

            [python]
            max_cyclomatic = 12
            made_up = 1
        "#,
    )
    .unwrap_err()
    .to_string();

    assert!(
        err.contains("unknown [smells] key(s): typo_toggle"),
        "{err}"
    );
    assert!(
        err.contains("unknown [smells.python] key(s): made_up"),
        "{err}"
    );
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
