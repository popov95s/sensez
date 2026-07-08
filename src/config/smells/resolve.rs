use super::defaults;
use super::knobs::Smells;
use super::raw::{split_rules, RuleRaw, SmellsRaw};
use super::rules::apply_rules;
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

fn language_label(lang: Language) -> &'static str {
    match lang {
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::Rust => "rust",
    }
}
