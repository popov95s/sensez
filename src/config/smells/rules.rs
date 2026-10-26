use super::knobs::{Smells, Strictness};
use super::raw::RuleRaw;
use super::rule_lists::{globs, string_list};
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
            set_rule_enabled(smells, kind, enabled);
        } else if recognized || rule.action.is_some() {
            set_rule_enabled(smells, kind, true);
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
    if list_rule_knobs(kind).contains(&key) {
        return apply_list_knob(smells, kind, key, value);
    }
    if let Some(s) = value.as_str() {
        return apply_string_knob(smells, kind, key, s);
    }
    if strictness::is_string_rule_knob(kind, key) {
        return Err(format!("[smells.rules.{kind}] {key} must be a string"));
    }
    if let Some(b) = value.as_bool() {
        let matched = match (kind, key) {
            (SmellKind::MutatedParameter, "include_attributes") => {
                smells.param_attr_mutation = b;
                true
            }
            (SmellKind::RepeatedIteration, "include_tests") => {
                smells.repeated_iteration_include_tests = b;
                true
            }
            (SmellKind::LiteralMembership, "skip_external_vocab") => {
                smells.literal_membership_skip_external_vocab = b;
                true
            }
            (SmellKind::HeavyNestedFunction, "skip_capturing") => {
                smells.nested_function_skip_capturing = b;
                true
            }
            (SmellKind::TuplePacking, "allow_homogeneous") => {
                smells.tuple_allow_homogeneous = b;
                true
            }
            (SmellKind::NPlusOneCall, "await_in_loop") => {
                smells.n_plus_one_await_in_loop = b;
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
        return if rule_knob_names(kind).contains(&key) {
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

fn apply_list_knob(
    smells: &mut Smells,
    kind: SmellKind,
    key: &str,
    value: &toml::Value,
) -> Result<bool, String> {
    let items = string_list(value)
        .ok_or_else(|| format!("[smells.rules.{kind}] {key} must be a list of strings"))?;
    match (kind, key) {
        (SmellKind::MessageChain, "ignore_roots") => smells.message_chain_ignore_roots = items,
        (SmellKind::WeakTestOracle, "assertion_functions") => {
            smells.assertion_functions = globs(kind, key, items)?
        }
        (SmellKind::NPlusOneCall, "callees") => {
            smells.n_plus_one_callees = globs(kind, key, items)?
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn apply_string_knob(
    smells: &mut Smells,
    kind: SmellKind,
    key: &str,
    value: &str,
) -> Result<bool, String> {
    match (kind, key) {
        _ if strictness::is_string_rule_knob(kind, key) => set_strictness_knob(
            smells,
            kind,
            strictness::parse_strictness(kind, key, value)?,
        ),
        _ if rule_knob_names(kind).contains(&key) => {
            Err(format!("[smells.rules.{kind}] {key} must be an integer"))
        }
        _ => Ok(false),
    }
}

fn set_strictness_knob(
    smells: &mut Smells,
    kind: SmellKind,
    value: Strictness,
) -> Result<bool, String> {
    match kind {
        SmellKind::LooseTyping => {
            smells.loose_typing_strictness = value;
            Ok(true)
        }
        SmellKind::MagicStringDefault => {
            smells.magic_string_default_strictness = value;
            Ok(true)
        }
        SmellKind::NestedTernary => {
            smells.nested_ternary_strictness = value;
            Ok(true)
        }
        _ => Err(format!("[smells.rules.{kind}] has no strictness storage")),
    }
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
        (SmellKind::RedundantWrapperChain, "max_depth") => set(&mut smells.max_wrapper_depth, n),
        (SmellKind::DataClump, "min_fields") => set(&mut smells.data_clump_min_fields, n),
        (SmellKind::DataClump, "min_occurrences") => set(&mut smells.data_clump_min_occurrences, n),
        (SmellKind::ShotgunSurgeryHazard, "min_blast") => {
            set(&mut smells.shotgun_blast_threshold, n)
        }
        (SmellKind::GodModule, "min_fan") => set(&mut smells.god_module_fan, n),
        (SmellKind::GodModule, "min_fan_in") => set(&mut smells.god_module_min_fan_in, n),
        (SmellKind::GodModule, "min_fan_out") => set(&mut smells.god_module_min_fan_out, n),
        (SmellKind::GodModule, "min_lines") => set(&mut smells.god_module_min_lines, n),
        (SmellKind::GodModule, "min_units") => set(&mut smells.god_module_min_units, n),
        (SmellKind::NarratingCode, "min_comment_lines") => set(&mut smells.min_comment_lines, n),
        (SmellKind::NarratingCode, "max_comment_ratio_percent") => {
            set(&mut smells.max_comment_ratio_percent, n)
        }
        (SmellKind::NarratingCode, "min_function_lines") => {
            set(&mut smells.narrating_min_function_lines, n)
        }
        (SmellKind::BooleanBlindness, "max_bool_params") => set(&mut smells.max_bool_params, n),
        (SmellKind::TuplePacking, "max_tuple_return") => set(&mut smells.max_tuple_return, n),
        (SmellKind::ImplicitSchema, "min_keys") => set(&mut smells.implicit_schema_min_keys, n),
        (SmellKind::HeavyNestedFunction, "max_lines") => {
            set(&mut smells.max_nested_function_lines, n)
        }
        (SmellKind::NPlusOneCall, "max_constant_bound") => {
            set(&mut smells.n_plus_one_max_constant_bound, n)
        }
        _ => false,
    }
}

fn set(slot: &mut usize, value: usize) -> bool {
    *slot = value;
    true
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
        .chain(rule_knob_names(kind).iter().copied())
        .chain(strictness_knobs)
        .collect()
}

fn rule_knob_names(kind: SmellKind) -> &'static [&'static str] {
    match kind {
        SmellKind::SplitVariable => &["min_assigns"],
        SmellKind::MutatedParameter => &["include_attributes"],
        SmellKind::LongFunction => &["max_lines"],
        SmellKind::HeavyNestedFunction => &["max_lines", "skip_capturing"],
        SmellKind::LargeClass => &["max_methods"],
        SmellKind::LongParameterList => &["max_params"],
        SmellKind::TooManyReturns => &["max_returns"],
        SmellKind::HighComplexity => &["max_cyclomatic"],
        SmellKind::HighCognitiveComplexity => &["max_cognitive"],
        SmellKind::DeepNesting => &["max_nesting"],
        SmellKind::MessageChain => &["max_depth", "ignore_roots"],
        SmellKind::RedundantWrapperChain => &["max_depth"],
        SmellKind::DataClump => &["min_fields", "min_occurrences"],
        SmellKind::ShotgunSurgeryHazard => &["min_blast"],
        SmellKind::GodModule => &[
            "min_fan",
            "min_fan_in",
            "min_fan_out",
            "min_lines",
            "min_units",
        ],
        SmellKind::NarratingCode => &[
            "min_comment_lines",
            "max_comment_ratio_percent",
            "min_function_lines",
        ],
        SmellKind::BooleanBlindness => &["max_bool_params"],
        SmellKind::MagicStringDefault => &[],
        SmellKind::RepeatedIteration => &["include_tests"],
        SmellKind::TuplePacking => &["max_tuple_return", "allow_homogeneous"],
        SmellKind::ImplicitSchema => &["min_keys"],
        SmellKind::LiteralMembership => &["skip_external_vocab"],
        SmellKind::WeakTestOracle => &["assertion_functions"],
        SmellKind::NPlusOneCall => &["callees", "max_constant_bound", "await_in_loop"],
        _ => &[],
    }
}

fn bool_rule_knobs(kind: SmellKind) -> &'static [&'static str] {
    match kind {
        SmellKind::MutatedParameter => &["include_attributes"],
        SmellKind::RepeatedIteration => &["include_tests"],
        SmellKind::LiteralMembership => &["skip_external_vocab"],
        SmellKind::HeavyNestedFunction => &["skip_capturing"],
        SmellKind::TuplePacking => &["allow_homogeneous"],
        SmellKind::NPlusOneCall => &["await_in_loop"],
        _ => &[],
    }
}

fn list_rule_knobs(kind: SmellKind) -> &'static [&'static str] {
    match kind {
        SmellKind::MessageChain => &["ignore_roots"],
        SmellKind::WeakTestOracle => &["assertion_functions"],
        SmellKind::NPlusOneCall => &["callees"],
        _ => &[],
    }
}

pub(super) fn set_rule_enabled(smells: &mut Smells, kind: SmellKind, enabled: bool) {
    match kind {
        SmellKind::MagicNumbers => smells.magic_numbers = enabled,
        SmellKind::NarratingCode => smells.narrating_code = enabled,
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
