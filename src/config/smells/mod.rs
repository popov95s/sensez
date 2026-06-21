//! Per-language design-smell configuration.

mod defaults;
mod knobs;

pub use knobs::Smells;

use crate::profiles::Language;
use crate::report::{ActionLevel, SmellKind};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(try_from = "SmellsRaw")]
pub struct SmellConfig {
    pub enabled: bool,
    pub exclude: Vec<String>,
    python: Smells,
    javascript: Smells,
    typescript: Smells,
    rust: Smells,
}

impl SmellConfig {
    /// The resolved knob set for `lang`.
    pub fn for_language(&self, lang: Language) -> &Smells {
        match lang {
            Language::Python => &self.python,
            Language::JavaScript => &self.javascript,
            Language::TypeScript => &self.typescript,
            Language::Rust => &self.rust,
        }
    }
}

impl Default for SmellConfig {
    fn default() -> Self {
        match SmellsRaw::default().try_into() {
            Ok(config) => config,
            Err(err) => panic!("built-in smell defaults are invalid: {err}"),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct SmellsRaw {
    enabled: Option<bool>,
    exclude: Vec<String>,
    rules: BTreeMap<SmellKind, RuleRaw>,
    python: toml::Table,
    javascript: toml::Table,
    typescript: toml::Table,
    rust: toml::Table,
    #[serde(flatten)]
    base: toml::Table,
}

impl TryFrom<SmellsRaw> for SmellConfig {
    type Error = String;

    fn try_from(raw: SmellsRaw) -> Result<Self, Self::Error> {
        let (python, python_rules) = split_rules("python", &raw.python)?;
        let (javascript, javascript_rules) = split_rules("javascript", &raw.javascript)?;
        let (typescript, typescript_rules) = split_rules("typescript", &raw.typescript)?;
        let (rust, rust_rules) = split_rules("rust", &raw.rust)?;
        let problems: Vec<String> = [
            validate_keys("<base>", &raw.base),
            validate_keys("python", &python),
            validate_keys("javascript", &javascript),
            validate_keys("typescript", &typescript),
            validate_keys("rust", &rust),
        ]
        .into_iter()
        .filter_map(Result::err)
        .collect();
        if !problems.is_empty() {
            return Err(problems.join("; "));
        }
        Ok(SmellConfig {
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
}

impl From<Smells> for SmellConfig {
    fn from(s: Smells) -> Self {
        SmellConfig {
            enabled: s.enabled,
            exclude: s.exclude.clone(),
            python: s.clone(),
            javascript: s.clone(),
            typescript: s.clone(),
            rust: s,
        }
    }
}

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

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct RuleRaw {
    enabled: Option<bool>,
    action: Option<ActionLevel>,
    #[serde(flatten)]
    knobs: toml::Table,
}

fn split_rules(
    scope: &str,
    table: &toml::Table,
) -> Result<(toml::Table, BTreeMap<SmellKind, RuleRaw>), String> {
    let mut outer = table.clone();
    let rules = match outer.remove("rules") {
        Some(toml::Value::Table(rules)) => toml::Value::Table(rules)
            .try_into()
            .map_err(|e| format!("invalid [smells.{scope}.rules]: {e}"))?,
        Some(_) => return Err(format!("[smells.{scope}.rules] must be a table")),
        None => BTreeMap::new(),
    };
    Ok((outer, rules))
}

fn apply_rules(
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
        SmellKind::LargeClass => smells.large_class = enabled,
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

fn validate_keys(scope: &str, table: &toml::Table) -> Result<(), String> {
    let allowed = allowed_keys();
    let unknown: Vec<_> = table
        .keys()
        .filter(|key| !allowed.contains(*key))
        .cloned()
        .collect();
    if unknown.is_empty() {
        Ok(())
    } else {
        let label = if scope == "<base>" {
            "[smells]".to_string()
        } else {
            format!("[smells.{scope}]")
        };
        Err(format!("unknown {label} key(s): {}", unknown.join(", ")))
    }
}

fn allowed_keys() -> BTreeSet<String> {
    match toml::Value::try_from(Smells::default()) {
        Ok(toml::Value::Table(table)) => table.keys().cloned().collect(),
        _ => BTreeSet::new(),
    }
}

#[cfg(test)]
mod tests;
