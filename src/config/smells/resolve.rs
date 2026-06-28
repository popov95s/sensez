use super::defaults;
use super::knobs::Smells;
use super::raw::{split_rules, RuleRaw, SmellsRaw};
use crate::config::smells::validate::validate_keys;
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
    let mut problems = Vec::new();
    collect_problem(&mut problems, validate_keys("<base>", &raw.base));
    collect_problem(&mut problems, validate_keys("python", &python));
    collect_problem(&mut problems, validate_keys("javascript", &javascript));
    collect_problem(&mut problems, validate_keys("typescript", &typescript));
    collect_problem(&mut problems, validate_keys("rust", &rust));
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    Ok(super::SmellConfig {
        enabled: raw.enabled.unwrap_or(true),
        exclude: raw.exclude,
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

fn collect_problem(problems: &mut Vec<String>, result: Result<(), String>) {
    if let Err(problem) = result {
        problems.push(problem);
    }
}

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
    apply_rules(&mut smells, "<base>", base_rules)?;
    apply_rules(&mut smells, language_label(lang), over_rules)?;
    Ok(smells)
}

/// Apply one `rules` scope's knobs/actions/enabled flags to `smells`.
pub(super) fn apply_rules(
    smells: &mut Smells,
    scope: &str,
    rules: &BTreeMap<SmellKind, RuleRaw>,
) -> Result<(), String> {
    for (&kind, rule) in rules {
        let recognized = apply_rule_knobs(smells, kind, &rule.knobs)?;
        if let Some(action) = rule.action {
            smells.actions.insert(kind, action);
        }
        if let Some(enabled) = rule.enabled {
            set_rule_enabled(smells, kind, enabled);
        } else if recognized {
            set_rule_enabled(smells, kind, true);
        }

        let unknown: Vec<_> = rule
            .knobs
            .keys()
            .filter(|key| !is_rule_key(kind, key))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            return Err(format!(
                "unknown [smells.{scope}.rules.{kind}] key(s): {}",
                unknown.join(", ")
            ));
        }
    }
    Ok(())
}

fn apply_rule_knobs(
    smells: &mut Smells,
    kind: SmellKind,
    knobs: &toml::Table,
) -> Result<bool, String> {
    let mut recognized = false;
    for (key, value) in knobs {
        recognized |= apply_rule_knob(smells, kind, key, value)?;
    }
    Ok(recognized)
}

fn apply_rule_knob(
    smells: &mut Smells,
    kind: SmellKind,
    key: &str,
    value: &toml::Value,
) -> Result<bool, String> {
    if let Some(b) = value.as_bool() {
        let matched = match (kind, key) {
            (SmellKind::MutatedParameter, "include_attributes") => {
                smells.param_attr_mutation = b;
                true
            }
            _ => false,
        };
        return Ok(matched);
    }
    if bool_rule_knobs(kind).contains(&key) {
        return Err(format!("[smells.rules.{kind}] {key} must be a boolean"));
    }
    let Some(n) = value.as_integer() else {
        return if rule_knobs(kind).contains(&key) {
            Err(format!("[smells.rules.{kind}] {key} must be an integer"))
        } else {
            Ok(false)
        };
    };
    if n < 0 {
        return Err(format!("[smells.rules.{kind}] {key} must be non-negative"));
    }
    let n = n as usize;
    let matched = match (kind, key) {
        (SmellKind::SplitVariable, "min_assigns") => {
            smells.split_variable_min_assigns = n;
            true
        }
        (SmellKind::LongFunction, "max_lines") => {
            smells.max_function_lines = n;
            true
        }
        (SmellKind::LargeClass, "max_methods") => {
            smells.max_class_methods = n;
            true
        }
        (SmellKind::LongParameterList, "max_params") => {
            smells.max_params = n;
            true
        }
        (SmellKind::TooManyReturns, "max_returns") => {
            smells.max_returns = n;
            true
        }
        (SmellKind::HighComplexity, "max_cyclomatic") => {
            smells.max_cyclomatic = n;
            true
        }
        (SmellKind::HighCognitiveComplexity, "max_cognitive") => {
            smells.max_cognitive = n;
            true
        }
        (SmellKind::DeepNesting, "max_nesting") => {
            smells.max_nesting = n;
            true
        }
        (SmellKind::MessageChain, "max_depth") => {
            smells.max_chain_depth = n;
            true
        }
        (SmellKind::DataClump, "min_fields") => {
            smells.data_clump_min_fields = n;
            true
        }
        (SmellKind::DataClump, "min_occurrences") => {
            smells.data_clump_min_occurrences = n;
            true
        }
        (SmellKind::ShotgunSurgeryHazard, "min_blast") => {
            smells.shotgun_blast_threshold = n;
            true
        }
        (SmellKind::GodModule, "min_fan") => {
            smells.god_module_fan = n;
            true
        }
        (SmellKind::BooleanBlindness, "max_bool_params") => {
            smells.max_bool_params = n;
            true
        }
        (SmellKind::TuplePacking, "max_tuple_return") => {
            smells.max_tuple_return = n;
            true
        }
        (SmellKind::ImplicitSchema, "min_keys") => {
            smells.implicit_schema_min_keys = n;
            true
        }
        (SmellKind::HeavyNestedFunction, "max_lines") => {
            smells.max_nested_function_lines = n;
            true
        }
        _ => false,
    };
    Ok(matched)
}

fn is_rule_key(kind: SmellKind, key: &str) -> bool {
    matches!(key, "enabled" | "action") || rule_knobs(kind).contains(&key)
}

fn rule_knobs(kind: SmellKind) -> &'static [&'static str] {
    match kind {
        SmellKind::SplitVariable => &["min_assigns"],
        SmellKind::MutatedParameter => &["include_attributes"],
        SmellKind::LongFunction | SmellKind::HeavyNestedFunction => &["max_lines"],
        SmellKind::LargeClass => &["max_methods"],
        SmellKind::LongParameterList => &["max_params"],
        SmellKind::TooManyReturns => &["max_returns"],
        SmellKind::HighComplexity => &["max_cyclomatic"],
        SmellKind::HighCognitiveComplexity => &["max_cognitive"],
        SmellKind::DeepNesting => &["max_nesting"],
        SmellKind::MessageChain => &["max_depth"],
        SmellKind::DataClump => &["min_fields", "min_occurrences"],
        SmellKind::ShotgunSurgeryHazard => &["min_blast"],
        SmellKind::GodModule => &["min_fan"],
        SmellKind::BooleanBlindness => &["max_bool_params"],
        SmellKind::MagicStringDefault => &[],
        SmellKind::TuplePacking => &["max_tuple_return"],
        SmellKind::ImplicitSchema => &["min_keys"],
        _ => &[],
    }
}

fn bool_rule_knobs(kind: SmellKind) -> &'static [&'static str] {
    match kind {
        SmellKind::MutatedParameter => &["include_attributes"],
        _ => &[],
    }
}

fn set_rule_enabled(smells: &mut Smells, kind: SmellKind, enabled: bool) {
    match kind {
        SmellKind::MagicNumbers => smells.magic_numbers = enabled,
        SmellKind::HighComplexity => smells.cyclomatic_complexity = enabled,
        SmellKind::LongFunction => smells.long_function = enabled,
        SmellKind::LongParameterList => smells.long_parameter_list = enabled,
        SmellKind::TooManyReturns => smells.too_many_returns = enabled,
        SmellKind::SplitVariable => smells.split_variable = enabled,
        SmellKind::LooseTyping => smells.loose_typing = enabled,
        SmellKind::MagicStringDefault => smells.magic_string_default = enabled,
        SmellKind::TuplePacking => smells.tuple_packing = enabled,
        SmellKind::MutatedParameter => smells.param_mutation = enabled,
        SmellKind::ReassignedParameter => smells.param_reassignment = enabled,
        SmellKind::LiteralMembership => smells.literal_membership = enabled,
        _ => {}
    }
    if enabled {
        smells.disabled.retain(|disabled| *disabled != kind);
    } else if !smells.disabled.contains(&kind) {
        smells.disabled.push(kind);
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
