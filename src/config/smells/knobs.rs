//! Per-language smell thresholds and toggles.

use crate::report::{ActionLevel, SmellKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Strictness {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Smells {
    pub enabled: bool,
    pub exclude: Vec<String>,
    pub disabled: Vec<SmellKind>,
    #[serde(skip)]
    pub actions: BTreeMap<SmellKind, ActionLevel>,
    pub max_cyclomatic: usize,
    pub max_cognitive: usize,
    pub max_function_lines: usize,
    pub max_nesting: usize,
    pub max_params: usize,
    pub max_returns: usize,
    pub max_class_methods: usize,
    pub max_chain_depth: usize,
    pub data_clump_min_fields: usize,
    pub data_clump_min_occurrences: usize,
    pub shotgun_blast_threshold: usize,
    pub god_module_fan: usize,
    pub magic_numbers: bool,
    pub narrating_code: bool,
    pub min_comment_lines: usize,
    pub max_comment_ratio_percent: usize,
    pub cyclomatic_complexity: bool,
    pub long_function: bool,
    pub large_class: bool,
    pub long_parameter_list: bool,
    pub too_many_returns: bool,
    pub split_variable: bool,
    pub split_variable_min_assigns: usize,
    pub loose_typing: bool,
    pub loose_typing_strictness: Strictness,
    pub magic_string_default: bool,
    pub max_bool_params: usize,
    pub tuple_packing: bool,
    pub max_tuple_return: usize,
    pub param_mutation: bool,
    pub param_attr_mutation: bool,
    pub param_reassignment: bool,
    pub implicit_schema_min_keys: usize,
    pub literal_membership: bool,
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
            magic_numbers: false,
            narrating_code: true,
            min_comment_lines: 5,
            max_comment_ratio_percent: 30,
            cyclomatic_complexity: false,
            long_function: false,
            large_class: false,
            long_parameter_list: false,
            too_many_returns: false,
            split_variable: false,
            split_variable_min_assigns: 2,
            loose_typing: true,
            loose_typing_strictness: Strictness::Medium,
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
