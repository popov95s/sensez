//! Unknown-key warnings for sensez configuration tables.

use super::model::{
    ActionPolicy, Boundaries, Cache, Config, Cycles, DeadCode, Duplication, Gate,
    SemanticDuplication,
};
use super::SelfImprovement;
use rustc_hash::FxHashSet;
use std::collections::BTreeSet;

/// Allowed keys inside one `[[boundaries.forbidden]]` entry.
const FORBIDDEN_KEYS: [&str; 2] = ["from", "to"];

pub(super) fn collect_unknown_keys(table: &toml::Table) -> Vec<String> {
    let mut out = BTreeSet::new();
    check("", table, &allowed_keys::<Config>(), &mut out);

    if let Some(t) = sub_table(table, "cache") {
        check("cache", t, &allowed_keys::<Cache>(), &mut out);
    }
    if let Some(t) = sub_table(table, "duplication") {
        check("duplication", t, &allowed_keys::<Duplication>(), &mut out);
        if let Some(s) = sub_table(t, "semantic") {
            check(
                "duplication.semantic",
                s,
                &allowed_keys::<SemanticDuplication>(),
                &mut out,
            );
        }
    }
    if let Some(t) = sub_table(table, "dead_code") {
        check("dead_code", t, &allowed_keys::<DeadCode>(), &mut out);
    }
    if let Some(t) = sub_table(table, "cycles") {
        check("cycles", t, &allowed_keys::<Cycles>(), &mut out);
    }
    if let Some(t) = sub_table(table, "boundaries") {
        check("boundaries", t, &allowed_keys::<Boundaries>(), &mut out);
        let forbidden_allowed: FxHashSet<String> = FORBIDDEN_KEYS
            .iter()
            .map(|key| (*key).to_string())
            .collect();
        if let Some(rules) = t.get("forbidden").and_then(|v| v.as_array()) {
            for (idx, rule) in rules.iter().enumerate() {
                if let Some(rt) = rule.as_table() {
                    check(
                        &format!("boundaries.forbidden[{idx}]"),
                        rt,
                        &forbidden_allowed,
                        &mut out,
                    );
                }
            }
        }
    }
    if let Some(t) = sub_table(table, "action") {
        check("action", t, &allowed_keys::<ActionPolicy>(), &mut out);
    }
    if let Some(t) = sub_table(table, "gate") {
        check("gate", t, &allowed_keys::<Gate>(), &mut out);
    }
    if let Some(t) = sub_table(table, "self_improvement") {
        check(
            "self_improvement",
            t,
            &allowed_keys::<SelfImprovement>(),
            &mut out,
        );
    }

    out.into_iter().collect()
}

fn sub_table<'a>(table: &'a toml::Table, key: &str) -> Option<&'a toml::Table> {
    table.get(key).and_then(|v| v.as_table())
}

fn check(
    scope: &str,
    table: &toml::Table,
    allowed: &FxHashSet<String>,
    out: &mut BTreeSet<String>,
) {
    for key in table.keys() {
        if !allowed.contains(key) {
            out.insert(qualify(scope, key));
        }
    }
}

fn qualify(scope: &str, key: &str) -> String {
    if scope.is_empty() {
        key.to_string()
    } else {
        format!("{scope}.{key}")
    }
}

fn allowed_keys<T: Default + serde::Serialize>() -> FxHashSet<String> {
    match toml::Value::try_from(T::default()) {
        Ok(toml::Value::Table(table)) => table.keys().cloned().collect(),
        _ => FxHashSet::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(text: &str) -> Vec<String> {
        let value: toml::Value = text.parse().unwrap();
        collect_unknown_keys(value.as_table().unwrap())
    }

    #[test]
    fn valid_config_has_no_unknown_keys() {
        assert!(keys("[duplication]\nthreshold = 50\n[dead_code]\nunused_methods = true\n[gate]\nrepeat_limit = 3\n").is_empty());
    }

    #[test]
    fn typos_are_qualified_by_section() {
        let unknown = keys("treshold = 5\n[duplication]\ntreshold = 40\n");
        assert_eq!(unknown, vec!["duplication.treshold", "treshold"]);
    }

    #[test]
    fn semantic_and_forbidden_entries_are_checked() {
        let unknown = keys(
            "[duplication.semantic]\nmin_shape = 80\n[[boundaries.forbidden]]\nfrm = \"a\"\nto = \"b\"\n",
        );
        assert_eq!(
            unknown,
            vec![
                "boundaries.forbidden[0].frm",
                "duplication.semantic.min_shape"
            ]
        );
    }

    #[test]
    fn free_form_sections_are_not_flagged() {
        let unknown =
            keys("[accept]\nanything_at_all = [\"x\"]\n[action.smells]\nwhatever = \"info\"\n");
        assert!(unknown.is_empty());
    }
}
