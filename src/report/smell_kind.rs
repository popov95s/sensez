use serde::{Deserialize, Serialize};

/// Every design-smell family Sensez detects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmellKind {
    BooleanBlindness,
    DataClump,
    DeepNesting,
    DivergentChange,
    FeatureEnvy,
    GodModule,
    HeavyNestedFunction,
    HighCognitiveComplexity,
    HighComplexity,
    ImplicitSchema,
    InappropriateIntimacy,
    LargeClass,
    LiteralMembership,
    LongFunction,
    LongParameterList,
    LooseTyping,
    MagicStringDefault,
    MagicNumbers,
    MessageChain,
    MutatedParameter,
    NestedLoop,
    NPlusOneCall,
    ReassignedParameter,
    RefusedBequest,
    RepeatedIteration,
    ShotgunSurgeryHazard,
    SplitVariable,
    SortInLoop,
    TooManyReturns,
    TuplePacking,
    UnnecessaryNestedIf,
}

impl SmellKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SmellKind::BooleanBlindness => "boolean_blindness",
            SmellKind::DataClump => "data_clump",
            SmellKind::DeepNesting => "deep_nesting",
            SmellKind::DivergentChange => "divergent_change",
            SmellKind::FeatureEnvy => "feature_envy",
            SmellKind::GodModule => "god_module",
            SmellKind::HeavyNestedFunction => "heavy_nested_function",
            SmellKind::HighCognitiveComplexity => "high_cognitive_complexity",
            SmellKind::HighComplexity => "high_complexity",
            SmellKind::ImplicitSchema => "implicit_schema",
            SmellKind::InappropriateIntimacy => "inappropriate_intimacy",
            SmellKind::LargeClass => "large_class",
            SmellKind::LiteralMembership => "literal_membership",
            SmellKind::LongFunction => "long_function",
            SmellKind::LongParameterList => "long_parameter_list",
            SmellKind::LooseTyping => "loose_typing",
            SmellKind::MagicStringDefault => "magic_string_default",
            SmellKind::MagicNumbers => "magic_numbers",
            SmellKind::MessageChain => "message_chain",
            SmellKind::MutatedParameter => "mutated_parameter",
            SmellKind::NestedLoop => "nested_loop",
            SmellKind::NPlusOneCall => "n_plus_one_call",
            SmellKind::ReassignedParameter => "reassigned_parameter",
            SmellKind::RefusedBequest => "refused_bequest",
            SmellKind::RepeatedIteration => "repeated_iteration",
            SmellKind::ShotgunSurgeryHazard => "shotgun_surgery_hazard",
            SmellKind::SplitVariable => "split_variable",
            SmellKind::SortInLoop => "sort_in_loop",
            SmellKind::TooManyReturns => "too_many_returns",
            SmellKind::TuplePacking => "tuple_packing",
            SmellKind::UnnecessaryNestedIf => "unnecessary_nested_if",
        }
    }
}

impl std::fmt::Display for SmellKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
