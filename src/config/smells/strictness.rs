//! Shared metadata for rule knobs with ordered strictness levels.

use super::knobs::Strictness;
use crate::report::SmellKind;

#[allow(dead_code)] // Read by docs/rust_metadata.py as source-level metadata.
pub struct StrictnessLevelDoc {
    pub title: &'static str,
    pub value: &'static str,
    pub description: &'static str,
}

pub struct StrictnessRuleDoc {
    pub kind: SmellKind,
    pub knob: &'static str,
    pub levels: &'static [StrictnessLevelDoc],
}

pub const LOOSE_TYPING_LEVELS: &[StrictnessLevelDoc] = &[
    StrictnessLevelDoc {
        title: "Low",
        value: "low",
        description: "Flags only direct escape hatches.",
    },
    StrictnessLevelDoc {
        title: "Medium",
        value: "medium",
        description: "Flags escape hatches plus schema-erasing maps and records.",
    },
    StrictnessLevelDoc {
        title: "High",
        value: "high",
        description: "Flags everything in medium, primitive-only collections, and shallow aliases.",
    },
];

pub const STRICTNESS_RULES: &[StrictnessRuleDoc] = &[StrictnessRuleDoc {
    kind: SmellKind::LooseTyping,
    knob: "strictness",
    levels: LOOSE_TYPING_LEVELS,
}];

pub fn is_string_rule_knob(kind: SmellKind, key: &str) -> bool {
    strictness_rule(kind, key).is_some()
}

pub fn valid_values(kind: SmellKind, key: &str) -> Option<String> {
    let values = strictness_rule(kind, key)?
        .levels
        .iter()
        .map(|level| level.value)
        .collect::<Vec<_>>()
        .join(", ");
    Some(values)
}

pub fn parse_strictness(kind: SmellKind, key: &str, value: &str) -> Result<Strictness, String> {
    let Some(rule) = strictness_rule(kind, key) else {
        return Err(format!(
            "[smells.rules.{kind}] {key} is not a strictness knob"
        ));
    };
    match level_index(rule.levels, value) {
        Some(0) => Ok(Strictness::Low),
        Some(1) => Ok(Strictness::Medium),
        Some(2) => Ok(Strictness::High),
        _ => Err(format!(
            "[smells.rules.{kind}] {key} must be one of: {}",
            valid_values(kind, key).unwrap_or_default()
        )),
    }
}

fn level_index(levels: &[StrictnessLevelDoc], value: &str) -> Option<usize> {
    levels.iter().position(|level| level.value == value)
}

fn strictness_rule(kind: SmellKind, key: &str) -> Option<&'static StrictnessRuleDoc> {
    STRICTNESS_RULES
        .iter()
        .find(|rule| rule.kind == kind && rule.knob == key)
}
