use crate::report::SmellKind;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct SmellsRaw {
    pub(super) enabled: Option<bool>,
    pub(super) exclude: Vec<String>,
    pub(super) rules: BTreeMap<SmellKind, RuleRaw>,
    pub(super) python: toml::Table,
    pub(super) javascript: toml::Table,
    pub(super) typescript: toml::Table,
    pub(super) rust: toml::Table,
    #[serde(flatten)]
    pub(super) base: toml::Table,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub(super) struct RuleRaw {
    pub(super) enabled: Option<bool>,
    pub(super) action: Option<crate::report::ActionLevel>,
    #[serde(flatten)]
    pub(super) knobs: toml::Table,
}

/// Pop the `rules` sub-table off `table`, returning (the remaining per-
/// language table, the extracted rules). Reports a typed error so the
/// caller can attach the language scope.
pub(super) fn split_rules(
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
