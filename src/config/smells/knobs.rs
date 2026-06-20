//! The per-language smell knob set: thresholds + opt-in toggles + the uniform
//! `disabled` switch. One resolved [`Smells`] is handed to the detectors per
//! language (see [`super::SmellConfig`]).

use crate::noze::{ActionLevel, SmellKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Thresholds for the always-on metric smells plus opt-in toggles for the
/// heuristic/advisory ones (off by default). `Serialize` is required so the
/// per-language resolver can round-trip a default through a `toml::Table` to
/// overlay user keys (see [`super::SmellConfig`]).
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Smells {
    /// Master switch for the whole pillar.
    pub enabled: bool,
    /// File globs excluded from smell analysis (defaults to tests/migrations).
    pub exclude: Vec<String>,
    /// Smell kinds switched off for this language (the uniform per-smell on/off,
    /// applied after detection regardless of any per-smell threshold/toggle).
    pub disabled: Vec<SmellKind>,
    /// Per-smell action overrides resolved from `[smells.rules.*]`.
    #[serde(skip)]
    pub actions: BTreeMap<SmellKind, ActionLevel>,
    pub max_cyclomatic: usize,
    pub max_cognitive: usize,
    pub max_function_lines: usize,
    pub max_nesting: usize,
    pub max_params: usize,
    pub max_returns: usize,
    pub max_class_methods: usize,
    /// Longest tolerated `a.b.c.d` attribute chain (Law of Demeter).
    pub max_chain_depth: usize,
    /// Data-clump mining: minimum bundle size (number of fields that must
    /// travel together; floored at 2) and minimum recurrence count.
    pub data_clump_min_fields: usize,
    pub data_clump_min_occurrences: usize,
    /// Shotgun-Surgery Hazard: minimum distinct dependents to flag a module.
    pub shotgun_blast_threshold: usize,
    /// God Module: minimum combined fan-in + fan-out to flag a module.
    pub god_module_fan: usize,
    /// Metric smells commonly owned by language linters — off by default to
    /// avoid duplicate reports. Enable per language to use Sensez' version.
    pub magic_numbers: bool,
    pub cyclomatic_complexity: bool,
    pub long_function: bool,
    pub large_class: bool,
    pub long_parameter_list: bool,
    pub too_many_returns: bool,
    /// Opt-in advisory (heuristic, may be noisy) smells.
    pub split_variable: bool,
    /// Min plain assignments to one local before `split_variable` flags it
    /// (floored at 2). At 2 it enforces single-binding locals: a name assigned
    /// in both arms of an if/else flags — extract a helper returning the value.
    pub split_variable_min_assigns: usize,
    /// Type-discipline family (annotation-gated; on by default).
    pub loose_typing: bool,
    /// Fallback empty/one-char strings used to hide optionality behind a
    /// mandatory string contract (`or ""`, `|| "?"`).
    pub magic_string_default: bool,
    /// Flag functions with more than this many bool-annotated params.
    pub max_bool_params: usize,
    pub tuple_packing: bool,
    /// Flag tuple returns with more elements than this.
    pub max_tuple_return: usize,
    /// Mutation/stringly-typed discipline family.
    pub param_mutation: bool,
    /// Opt-in (stricter): also flag mutation reached *through* a parameter's
    /// attribute — `msg.kwargs[k]=v`, `msg.items.append(...)`. Off by default
    /// because it also surfaces idiomatic framework mutation (`request.session`).
    /// `self`/`cls` and locals are never flagged regardless.
    pub param_attr_mutation: bool,
    /// Opt-in: rebinding a param (`x = x or []` is idiomatic, so off).
    pub param_reassignment: bool,
    /// Min distinct string keys on one receiver to flag (0 disables).
    pub implicit_schema_min_keys: usize,
    pub literal_membership: bool,
    /// Max lines tolerated in a function nested inside another function before
    /// it stops being a "simple wrapper" (0 disables).
    pub max_nested_function_lines: usize,
}

impl Default for Smells {
    fn default() -> Self {
        Smells {
            enabled: true,
            exclude: Vec::new(),
            disabled: Vec::new(),
            actions: BTreeMap::new(),
            max_cyclomatic: 10,
            max_cognitive: 15,
            max_function_lines: 50,
            max_nesting: 4,
            max_params: 5,
            max_returns: 4,
            max_class_methods: 20,
            max_chain_depth: 4,
            data_clump_min_fields: 4,
            data_clump_min_occurrences: 3,
            shotgun_blast_threshold: 4,
            god_module_fan: 25,
            // Linter-owned: disabled by default (see field docs).
            magic_numbers: false,
            cyclomatic_complexity: false,
            long_function: false,
            large_class: false,
            long_parameter_list: false,
            too_many_returns: false,
            split_variable: false,
            split_variable_min_assigns: 2,
            loose_typing: true,
            magic_string_default: true,
            max_bool_params: 2,
            tuple_packing: true,
            max_tuple_return: 2,
            param_mutation: true,
            param_attr_mutation: false,
            param_reassignment: false,
            implicit_schema_min_keys: 4,
            literal_membership: true,
            max_nested_function_lines: 15,
        }
    }
}
