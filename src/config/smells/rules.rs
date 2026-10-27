use super::knob_table::{knob, knob_error, knob_names};
use super::knobs::{Smells, Strictness};
use super::raw::RuleRaw;
use super::strictness;
use crate::report::SmellKind;
use std::collections::BTreeMap;

/// Apply one `rules` scope's knobs/actions/enabled flags to `smells`.
pub(super) fn apply_rules(
    smells: &mut Smells,
    rules: &BTreeMap<SmellKind, RuleRaw>,
) -> Result<(), String> {
    for (&kind, rule) in rules {
        let recognized = rule
            .knobs
            .iter()
            .try_fold(false, |recognized, (key, value)| {
                apply_rule_knob(smells, kind, key, value).map(|matched| recognized | matched)
            })?;
        if let Some(action) = rule.action {
            smells.actions.insert(kind, action);
        }
        if let Some(enabled) = rule.enabled {
            smells.set_enabled(kind, enabled);
        } else if recognized || rule.action.is_some() {
            smells.set_enabled(kind, true);
        }
    }
    Ok(())
}

fn apply_rule_knob(
    smells: &mut Smells,
    kind: SmellKind,
    key: &str,
    value: &toml::Value,
) -> Result<bool, String> {
    if strictness::is_string_rule_knob(kind, key) {
        let level = value
            .as_str()
            .ok_or_else(|| knob_error(kind, key, "a string"))?;
        return set_strictness_knob(
            smells,
            kind,
            strictness::parse_strictness(kind, key, level)?,
        );
    }
    let Some(knob) = knob(kind, key) else {
        return Ok(false);
    };
    knob.apply(smells, kind, key, value)?;
    Ok(true)
}

fn set_strictness_knob(
    smells: &mut Smells,
    kind: SmellKind,
    value: Strictness,
) -> Result<bool, String> {
    let slot = match kind {
        SmellKind::LooseTyping => &mut smells.loose_typing_strictness,
        SmellKind::MagicStringDefault => &mut smells.magic_string_default_strictness,
        SmellKind::NestedTernary => &mut smells.nested_ternary_strictness,
        SmellKind::SwallowedErrorSentinel => &mut smells.swallowed_error_strictness,
        SmellKind::MockedSubject => &mut smells.mocked_subject_strictness,
        _ => return Err(format!("[smells.rules.{kind}] has no strictness storage")),
    };
    *slot = value;
    Ok(true)
}

const COMMON_RULE_KEYS: [&str; 2] = ["enabled", "action"];

/// Every key a `[smells.rules.<kind>]` table accepts.
pub(crate) fn rule_keys(kind: SmellKind) -> Vec<&'static str> {
    let strictness_knobs = strictness::STRICTNESS_RULES
        .iter()
        .filter(|rule| rule.kind == kind)
        .map(|rule| rule.knob);
    COMMON_RULE_KEYS
        .into_iter()
        .chain(knob_names(kind))
        .chain(strictness_knobs)
        .collect()
}
