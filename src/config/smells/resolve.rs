use super::defaults;
use super::knobs::Smells;
use super::raw::{split_rules, RuleRaw, SmellsRaw};
use super::rules::{apply_rules, set_rule_enabled};
use crate::report::SmellKind;
use crate::spine::ir::Language;
use std::collections::BTreeMap;

/// Build a `SmellConfig` from a [`SmellsRaw`]: validate each scope, then
/// resolve every per-language [`Smells`] by overlaying the base table
/// followed by the per-language table, and finally applying the per-scope
/// `rules` table.
pub(super) fn resolve_config(raw: SmellsRaw) -> Result<super::SmellConfig, String> {
    let (python, python_rules) = split_rules("python", &raw.python)?;
    let (javascript, javascript_rules) = split_rules("javascript", &raw.javascript)?;
    let (typescript, typescript_rules) = split_rules("typescript", &raw.typescript)?;
    let (rust, rust_rules) = split_rules("rust", &raw.rust)?;
    Ok(super::SmellConfig {
        enabled: raw.enabled.unwrap_or(true),
        explicit_exclude: raw.exclude.clone(),
        exclude: raw.exclude,
        include_tests: raw.include_tests,
        python: resolve(
            Language::Python,
            &raw.base,
            &python,
            &raw.rules,
            &python_rules,
        )?,
        javascript: resolve(
            Language::JavaScript,
            &raw.base,
            &javascript,
            &raw.rules,
            &javascript_rules,
        )?,
        typescript: resolve(
            Language::TypeScript,
            &raw.base,
            &typescript,
            &raw.rules,
            &typescript_rules,
        )?,
        rust: resolve(Language::Rust, &raw.base, &rust, &raw.rules, &rust_rules)?,
    })
}

/// Flat `[smells]` thresholds and the rule each one belongs to.
const FLAT_THRESHOLDS: [(&str, SmellKind); 13] = [
    ("max_cyclomatic", SmellKind::HighComplexity),
    ("max_cognitive", SmellKind::HighCognitiveComplexity),
    ("max_function_lines", SmellKind::LongFunction),
    ("max_nesting", SmellKind::DeepNesting),
    ("max_params", SmellKind::LongParameterList),
    ("max_returns", SmellKind::TooManyReturns),
    ("max_class_methods", SmellKind::LargeClass),
    ("max_chain_depth", SmellKind::MessageChain),
    ("max_wrapper_depth", SmellKind::RedundantWrapperChain),
    ("max_bool_params", SmellKind::BooleanBlindness),
    ("max_tuple_return", SmellKind::TuplePacking),
    ("max_comment_ratio_percent", SmellKind::NarratingCode),
    ("max_nested_function_lines", SmellKind::HeavyNestedFunction),
];

/// Resolve one language's [`Smells`]: start from the per-language defaults,
/// overlay the base table, then the per-language table, then apply both
/// rule sets.
fn resolve(
    lang: Language,
    base: &toml::Table,
    over: &toml::Table,
    base_rules: &BTreeMap<SmellKind, RuleRaw>,
    over_rules: &BTreeMap<SmellKind, RuleRaw>,
) -> Result<Smells, String> {
    let default = defaults::default_for(lang);
    let mut table = match toml::Value::try_from(&default) {
        Ok(toml::Value::Table(t)) => t,
        _ => return Ok(default),
    };
    for (k, v) in base.iter().chain(over.iter()) {
        table.insert(k.clone(), v.clone());
    }
    let mut smells: Smells = toml::Value::Table(table)
        .try_into()
        .map_err(|err| format!("invalid [smells.{}]: {err}", language_label(lang)))?;
    for (key, kind) in FLAT_THRESHOLDS {
        if flat_threshold_enables(key, kind, base, over) {
            set_rule_enabled(&mut smells, kind, true);
        }
    }
    apply_rules(&mut smells, base_rules)?;
    apply_rules(&mut smells, over_rules)?;
    Ok(smells)
}

/// Whether a flat threshold enables its rule: the most specific table that
/// sets the threshold wins unless it, or a more specific one, sets the toggle.
fn flat_threshold_enables(
    key: &str,
    kind: SmellKind,
    base: &toml::Table,
    over: &toml::Table,
) -> bool {
    if over.contains_key(key) {
        return !sets_toggle(over, kind);
    }
    base.contains_key(key) && !sets_toggle(base, kind) && !sets_toggle(over, kind)
}

fn sets_toggle(table: &toml::Table, kind: SmellKind) -> bool {
    let listed_disabled = table
        .get("disabled")
        .and_then(toml::Value::as_array)
        .is_some_and(|kinds| kinds.iter().any(|k| k.as_str() == Some(kind.as_str())));
    listed_disabled || toggle_key(kind).is_some_and(|toggle| table.contains_key(toggle))
}

fn toggle_key(kind: SmellKind) -> Option<&'static str> {
    match kind {
        SmellKind::HighComplexity => Some("cyclomatic_complexity"),
        SmellKind::LongFunction => Some("long_function"),
        SmellKind::LargeClass => Some("large_class"),
        SmellKind::LongParameterList => Some("long_parameter_list"),
        SmellKind::TooManyReturns => Some("too_many_returns"),
        SmellKind::TuplePacking => Some("tuple_packing"),
        SmellKind::NarratingCode => Some("narrating_code"),
        _ => None,
    }
}

fn language_label(lang: Language) -> &'static str {
    match lang {
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::Rust => "rust",
    }
}
