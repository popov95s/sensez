use super::knobs::Smells;
use super::raw::RuleRaw;
use crate::report::SmellKind;
use std::collections::BTreeMap;

/// Apply one `rules` scope's knobs/actions/enabled flags to `smells`.
pub(super) fn apply_rules(
    smells: &mut Smells,
    scope: &str,
    rules: &BTreeMap<SmellKind, RuleRaw>,
) -> Result<(), String> {
    validate_rule_keys(scope, rules)?;
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
            set_rule_enabled(smells, kind, enabled);
        } else if recognized {
            set_rule_enabled(smells, kind, true);
        }
    }
    Ok(())
}

fn validate_rule_keys(scope: &str, rules: &BTreeMap<SmellKind, RuleRaw>) -> Result<(), String> {
    let unknown: Vec<_> = rules
        .iter()
        .flat_map(|(&kind, rule)| {
            rule.knobs
                .keys()
                .filter(move |key| !is_rule_key(kind, key))
                .map(move |key| format!("{kind}.{key}"))
        })
        .collect();
    if unknown.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "unknown [smells.{scope}.rules] key(s): {}",
            unknown.join(", ")
        ))
    }
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
    Ok(apply_integer_knob(smells, kind, key, n as usize))
}

fn apply_integer_knob(smells: &mut Smells, kind: SmellKind, key: &str, n: usize) -> bool {
    match (kind, key) {
        (SmellKind::SplitVariable, "min_assigns") => set(&mut smells.split_variable_min_assigns, n),
        (SmellKind::LongFunction, "max_lines") => set(&mut smells.max_function_lines, n),
        (SmellKind::LargeClass, "max_methods") => set(&mut smells.max_class_methods, n),
        (SmellKind::LongParameterList, "max_params") => set(&mut smells.max_params, n),
        (SmellKind::TooManyReturns, "max_returns") => set(&mut smells.max_returns, n),
        (SmellKind::HighComplexity, "max_cyclomatic") => set(&mut smells.max_cyclomatic, n),
        (SmellKind::HighCognitiveComplexity, "max_cognitive") => set(&mut smells.max_cognitive, n),
        (SmellKind::DeepNesting, "max_nesting") => set(&mut smells.max_nesting, n),
        (SmellKind::MessageChain, "max_depth") => set(&mut smells.max_chain_depth, n),
        (SmellKind::DataClump, "min_fields") => set(&mut smells.data_clump_min_fields, n),
        (SmellKind::DataClump, "min_occurrences") => set(&mut smells.data_clump_min_occurrences, n),
        (SmellKind::ShotgunSurgeryHazard, "min_blast") => {
            set(&mut smells.shotgun_blast_threshold, n)
        }
        (SmellKind::GodModule, "min_fan") => set(&mut smells.god_module_fan, n),
        (SmellKind::NarratingCode, "min_comment_lines") => set(&mut smells.min_comment_lines, n),
        (SmellKind::NarratingCode, "max_comment_ratio_percent") => {
            set(&mut smells.max_comment_ratio_percent, n)
        }
        (SmellKind::BooleanBlindness, "max_bool_params") => set(&mut smells.max_bool_params, n),
        (SmellKind::TuplePacking, "max_tuple_return") => set(&mut smells.max_tuple_return, n),
        (SmellKind::ImplicitSchema, "min_keys") => set(&mut smells.implicit_schema_min_keys, n),
        (SmellKind::HeavyNestedFunction, "max_lines") => {
            set(&mut smells.max_nested_function_lines, n)
        }
        _ => false,
    }
}

fn set(slot: &mut usize, value: usize) -> bool {
    *slot = value;
    true
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
        SmellKind::NarratingCode => &["min_comment_lines", "max_comment_ratio_percent"],
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
        SmellKind::NarratingCode => smells.narrating_code = enabled,
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
