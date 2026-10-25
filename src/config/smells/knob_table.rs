//! Every typed `[smells.rules.<kind>]` knob, declared once.

use super::knobs::Smells;
use super::rule_lists::{globs, number_list, string_list};
use crate::report::SmellKind;

/// A knob's value type and the `Smells` field it writes.
pub(super) enum Knob {
    Integer(fn(&mut Smells) -> &mut usize),
    Bool(fn(&mut Smells) -> &mut bool),
    Strings(fn(&mut Smells) -> &mut Vec<String>),
    Globs(fn(&mut Smells) -> &mut Vec<String>),
    Numbers(fn(&mut Smells) -> &mut Vec<String>),
}

use Knob::{Bool, Globs, Integer, Numbers, Strings};
use SmellKind as K;

pub(super) const RULE_KNOBS: &[(SmellKind, &str, Knob)] = &[
    (
        K::SplitVariable,
        "min_assigns",
        Integer(|s| &mut s.split_variable_min_assigns),
    ),
    (
        K::MutatedParameter,
        "include_attributes",
        Bool(|s| &mut s.param_attr_mutation),
    ),
    (
        K::LongFunction,
        "max_lines",
        Integer(|s| &mut s.max_function_lines),
    ),
    (
        K::HeavyNestedFunction,
        "max_lines",
        Integer(|s| &mut s.max_nested_function_lines),
    ),
    (
        K::HeavyNestedFunction,
        "skip_capturing",
        Bool(|s| &mut s.nested_function_skip_capturing),
    ),
    (
        K::LargeClass,
        "max_methods",
        Integer(|s| &mut s.max_class_methods),
    ),
    (
        K::LongParameterList,
        "max_params",
        Integer(|s| &mut s.max_params),
    ),
    (
        K::TooManyReturns,
        "max_returns",
        Integer(|s| &mut s.max_returns),
    ),
    (
        K::TooManyReturns,
        "count_guard_clauses",
        Bool(|s| &mut s.count_guard_clauses),
    ),
    (
        K::MagicNumbers,
        "ignore",
        Numbers(|s| &mut s.magic_number_ignore),
    ),
    (
        K::MagicNumbers,
        "ignore_array_indexes",
        Bool(|s| &mut s.magic_number_ignore_array_indexes),
    ),
    (
        K::MagicNumbers,
        "ignore_default_values",
        Bool(|s| &mut s.magic_number_ignore_default_values),
    ),
    (
        K::MagicNumbers,
        "allow_http_status",
        Bool(|s| &mut s.magic_number_allow_http_status),
    ),
    (
        K::HighComplexity,
        "max_cyclomatic",
        Integer(|s| &mut s.max_cyclomatic),
    ),
    (
        K::HighCognitiveComplexity,
        "max_cognitive",
        Integer(|s| &mut s.max_cognitive),
    ),
    (
        K::DeepNesting,
        "max_nesting",
        Integer(|s| &mut s.max_nesting),
    ),
    (
        K::MessageChain,
        "max_depth",
        Integer(|s| &mut s.max_chain_depth),
    ),
    (
        K::MessageChain,
        "ignore_roots",
        Strings(|s| &mut s.message_chain_ignore_roots),
    ),
    (
        K::RedundantWrapperChain,
        "max_depth",
        Integer(|s| &mut s.max_wrapper_depth),
    ),
    (
        K::DataClump,
        "min_fields",
        Integer(|s| &mut s.data_clump_min_fields),
    ),
    (
        K::DataClump,
        "min_occurrences",
        Integer(|s| &mut s.data_clump_min_occurrences),
    ),
    (
        K::SiblingHelperRedefinition,
        "min_similarity",
        Integer(|s| &mut s.sibling_helper_min_similarity),
    ),
    (
        K::SiblingHelperRedefinition,
        "min_lexemes",
        Integer(|s| &mut s.sibling_helper_min_lexemes),
    ),
    (
        K::ShotgunSurgeryHazard,
        "min_blast",
        Integer(|s| &mut s.shotgun_blast_threshold),
    ),
    (K::GodModule, "min_fan", Integer(|s| &mut s.god_module_fan)),
    (
        K::GodModule,
        "min_fan_in",
        Integer(|s| &mut s.god_module_min_fan_in),
    ),
    (
        K::GodModule,
        "min_fan_out",
        Integer(|s| &mut s.god_module_min_fan_out),
    ),
    (
        K::GodModule,
        "min_lines",
        Integer(|s| &mut s.god_module_min_lines),
    ),
    (
        K::GodModule,
        "min_units",
        Integer(|s| &mut s.god_module_min_units),
    ),
    (
        K::NarratingCode,
        "min_comment_lines",
        Integer(|s| &mut s.min_comment_lines),
    ),
    (
        K::NarratingCode,
        "max_comment_ratio_percent",
        Integer(|s| &mut s.max_comment_ratio_percent),
    ),
    (
        K::NarratingCode,
        "min_function_lines",
        Integer(|s| &mut s.narrating_min_function_lines),
    ),
    (
        K::BooleanBlindness,
        "max_bool_params",
        Integer(|s| &mut s.max_bool_params),
    ),
    (
        K::RepeatedIteration,
        "include_tests",
        Bool(|s| &mut s.repeated_iteration_include_tests),
    ),
    (
        K::TuplePacking,
        "max_tuple_return",
        Integer(|s| &mut s.max_tuple_return),
    ),
    (
        K::TuplePacking,
        "allow_homogeneous",
        Bool(|s| &mut s.tuple_allow_homogeneous),
    ),
    (
        K::ImplicitSchema,
        "min_keys",
        Integer(|s| &mut s.implicit_schema_min_keys),
    ),
    (
        K::LiteralMembership,
        "skip_external_vocab",
        Bool(|s| &mut s.literal_membership_skip_external_vocab),
    ),
    (
        K::WeakTestOracle,
        "assertion_functions",
        Globs(|s| &mut s.assertion_functions),
    ),
    (
        K::NPlusOneCall,
        "callees",
        Globs(|s| &mut s.n_plus_one_callees),
    ),
    (
        K::NPlusOneCall,
        "max_constant_bound",
        Integer(|s| &mut s.n_plus_one_max_constant_bound),
    ),
    (
        K::NPlusOneCall,
        "await_in_loop",
        Bool(|s| &mut s.n_plus_one_await_in_loop),
    ),
    (
        K::DependencyReimplementation,
        "providers",
        Strings(|s| &mut s.dependency_reimplementation_providers),
    ),
    (K::StubShipped, "markers", Strings(|s| &mut s.stub_markers)),
];

/// Typed knob names a rule accepts, in declaration order.
pub(super) fn knob_names(kind: SmellKind) -> impl Iterator<Item = &'static str> {
    RULE_KNOBS
        .iter()
        .filter(move |(owner, _, _)| *owner == kind)
        .map(|(_, name, _)| *name)
}

pub(super) fn knob(kind: SmellKind, key: &str) -> Option<&'static Knob> {
    RULE_KNOBS
        .iter()
        .find(|(owner, name, _)| *owner == kind && *name == key)
        .map(|(_, _, knob)| knob)
}

/// `[smells.rules.<kind>] <key> must be <expected>`.
pub(super) fn knob_error(kind: SmellKind, key: &str, expected: &str) -> String {
    format!("[smells.rules.{kind}] {key} must be {expected}")
}

impl Knob {
    fn expected(&self) -> &'static str {
        match self {
            Integer(_) => "an integer",
            Bool(_) => "a boolean",
            Strings(_) | Globs(_) => "a list of strings",
            Numbers(_) => "a list of numbers",
        }
    }

    /// Write `value` into its field, or explain why it does not fit.
    pub(super) fn apply(
        &self,
        smells: &mut Smells,
        kind: SmellKind,
        key: &str,
        value: &toml::Value,
    ) -> Result<(), String> {
        let wrong_type = || knob_error(kind, key, self.expected());
        match self {
            Integer(field) => {
                let n = value.as_integer().ok_or_else(wrong_type)?;
                *field(smells) =
                    usize::try_from(n).map_err(|_| knob_error(kind, key, "non-negative"))?;
            }
            Bool(field) => *field(smells) = value.as_bool().ok_or_else(wrong_type)?,
            Strings(field) => *field(smells) = string_list(value).ok_or_else(wrong_type)?,
            Globs(field) => {
                *field(smells) = globs(kind, key, string_list(value).ok_or_else(wrong_type)?)?
            }
            Numbers(field) => *field(smells) = number_list(value).ok_or_else(wrong_type)?,
        }
        Ok(())
    }
}
