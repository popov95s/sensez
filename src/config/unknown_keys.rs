//! Unknown-key detection for sensez configuration tables.

use super::field_names::accepted_fields;
use super::model::{
    ActionPolicy, Boundaries, Cache, Config, Cycles, DeadCode, Duplication, Gate,
    SemanticDuplication,
};
use super::smells::{rule_keys, Smells};
use super::suggest::suggest;
use super::SelfImprovement;
use crate::report::SmellKind;
use crate::spine::ir::Language;
use std::ops::Range;
use toml_edit::{Item, TableLike};

/// Allowed keys inside one `[[boundaries.forbidden]]` entry.
const FORBIDDEN_KEYS: [&str; 2] = ["from", "to"];

/// Whether an unknown key makes the configuration invalid or is only reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyPolicy {
    Reject,
    Warn,
}

pub(super) struct UnknownKey {
    pub(super) path: String,
    pub(super) span: Option<Range<usize>>,
    pub(super) suggestion: Option<String>,
    pub(super) policy: KeyPolicy,
}

pub(super) fn collect(root: &dyn TableLike) -> Vec<UnknownKey> {
    let mut out = Vec::new();
    check(
        "",
        root,
        &allowed_keys::<Config>(),
        KeyPolicy::Warn,
        &mut out,
    );
    let sections = [
        ("cache", allowed_keys::<Cache>()),
        ("dead_code", allowed_keys::<DeadCode>()),
        ("cycles", allowed_keys::<Cycles>()),
        ("gate", allowed_keys::<Gate>()),
        ("self_improvement", allowed_keys::<SelfImprovement>()),
    ];
    for (name, allowed) in &sections {
        if let Some(table) = sub_table(root, name) {
            check(name, table, allowed, KeyPolicy::Warn, &mut out);
        }
    }
    if let Some(table) = sub_table(root, "duplication") {
        check(
            "duplication",
            table,
            &allowed_keys::<Duplication>(),
            KeyPolicy::Warn,
            &mut out,
        );
        if let Some(semantic) = sub_table(table, "semantic") {
            check(
                "duplication.semantic",
                semantic,
                &allowed_keys::<SemanticDuplication>(),
                KeyPolicy::Warn,
                &mut out,
            );
        }
    }
    if let Some(table) = sub_table(root, "boundaries") {
        check(
            "boundaries",
            table,
            &allowed_keys::<Boundaries>(),
            KeyPolicy::Warn,
            &mut out,
        );
        for (idx, rule) in forbidden_entries(table).into_iter().enumerate() {
            check(
                &format!("boundaries.forbidden[{idx}]"),
                rule,
                &FORBIDDEN_KEYS.map(String::from),
                KeyPolicy::Warn,
                &mut out,
            );
        }
    }
    if let Some(table) = sub_table(root, "action") {
        check(
            "action",
            table,
            &allowed_keys::<ActionPolicy>(),
            KeyPolicy::Reject,
            &mut out,
        );
    }
    if let Some(table) = sub_table(root, "smells") {
        smells(table, &mut out);
    }
    out
}

fn smells(table: &dyn TableLike, out: &mut Vec<UnknownKey>) {
    let knobs = allowed_keys::<Smells>();
    let with_rules = || knobs.iter().cloned().chain(["rules".to_string()]);
    let base: Vec<String> = with_rules()
        .chain(Language::ALL.map(|lang| lang.key().to_string()))
        .chain(["include_tests".to_string()])
        .collect();
    check("smells", table, &base, KeyPolicy::Reject, out);
    rules("smells.rules", table, out);
    let language: Vec<String> = with_rules().collect();
    for lang in Language::ALL.map(Language::key) {
        if let Some(lang_table) = sub_table(table, lang) {
            let scope = format!("smells.{lang}");
            check(&scope, lang_table, &language, KeyPolicy::Reject, out);
            rules(&format!("{scope}.rules"), lang_table, out);
        }
    }
}

/// Rule names themselves are checked by deserialization (`SmellKind`).
fn rules(scope: &str, parent: &dyn TableLike, out: &mut Vec<UnknownKey>) {
    let Some(rules) = sub_table(parent, "rules") else {
        return;
    };
    for (name, item) in rules.iter() {
        let kind: Result<SmellKind, _> = toml::Value::String(name.to_string()).try_into();
        let (Some(rule), Ok(kind)) = (item.as_table_like(), kind) else {
            continue;
        };
        let allowed: Vec<String> = rule_keys(kind).into_iter().map(String::from).collect();
        check(
            &format!("{scope}.{name}"),
            rule,
            &allowed,
            KeyPolicy::Reject,
            out,
        );
    }
}

fn forbidden_entries(boundaries: &dyn TableLike) -> Vec<&dyn TableLike> {
    match boundaries.get("forbidden") {
        Some(Item::ArrayOfTables(tables)) => tables.iter().map(|t| t as &dyn TableLike).collect(),
        Some(Item::Value(toml_edit::Value::Array(values))) => values
            .iter()
            .filter_map(|value| value.as_inline_table())
            .map(|t| t as &dyn TableLike)
            .collect(),
        _ => Vec::new(),
    }
}

fn sub_table<'a>(table: &'a dyn TableLike, key: &str) -> Option<&'a dyn TableLike> {
    table.get(key).and_then(Item::as_table_like)
}

fn check(
    scope: &str,
    table: &dyn TableLike,
    allowed: &[String],
    policy: KeyPolicy,
    out: &mut Vec<UnknownKey>,
) {
    for (key, _) in table.iter() {
        if allowed.iter().any(|candidate| candidate == key) {
            continue;
        }
        out.push(UnknownKey {
            path: qualify(scope, key),
            span: table.get_key_value(key).and_then(|(k, _)| k.span()),
            suggestion: suggest(key, allowed.iter().map(String::as_str)).map(str::to_string),
            policy,
        });
    }
}

fn qualify(scope: &str, key: &str) -> String {
    if scope.is_empty() {
        key.to_string()
    } else {
        format!("{scope}.{key}")
    }
}

/// Serialized keys of `T`'s default, plus any deserialize-only aliases.
fn allowed_keys<T: Default + serde::Serialize + serde::de::DeserializeOwned>() -> Vec<String> {
    let mut keys = match toml::Value::try_from(T::default()) {
        Ok(toml::Value::Table(table)) => table.keys().cloned().collect(),
        _ => Vec::new(),
    };
    for field in accepted_fields::<T>() {
        if !keys.iter().any(|key| key == field) {
            keys.push((*field).to_string());
        }
    }
    keys
}

#[cfg(test)]
#[path = "unknown_keys_tests.rs"]
mod tests;
