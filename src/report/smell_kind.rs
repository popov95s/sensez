use serde::{Deserialize, Serialize};

/// Every design-smell family Sensez detects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmellKind {
    BooleanBlindness,
    DataClump,
    DeepNesting,
    DefensiveFallback,
    DependencyReimplementation,
    DivergentAbstraction,
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
    MockedSubject,
    MutatedParameter,
    NarratingCode,
    NestedLoop,
    NestedTernary,
    NPlusOneCall,
    ReassignedParameter,
    RedundantValidation,
    RedundantWrapperChain,
    RefusedBequest,
    RepeatedIteration,
    ShotgunSurgeryHazard,
    SiblingHelperRedefinition,
    SplitVariable,
    SortInLoop,
    SwallowedErrorSentinel,
    TooManyReturns,
    TuplePacking,
    UnnecessaryNestedIf,
    WeakTestOracle,
}

impl SmellKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SmellKind::BooleanBlindness => "boolean_blindness",
            SmellKind::DataClump => "data_clump",
            SmellKind::DeepNesting => "deep_nesting",
            SmellKind::DefensiveFallback => "defensive_fallback",
            SmellKind::DependencyReimplementation => "dependency_reimplementation",
            SmellKind::DivergentAbstraction => "divergent_abstraction",
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
            SmellKind::MockedSubject => "mocked_subject",
            SmellKind::MutatedParameter => "mutated_parameter",
            SmellKind::NarratingCode => "narrating_code",
            SmellKind::NestedLoop => "nested_loop",
            SmellKind::NestedTernary => "nested_ternary",
            SmellKind::NPlusOneCall => "n_plus_one_call",
            SmellKind::ReassignedParameter => "reassigned_parameter",
            SmellKind::RedundantValidation => "redundant_validation",
            SmellKind::RedundantWrapperChain => "redundant_wrapper_chain",
            SmellKind::RefusedBequest => "refused_bequest",
            SmellKind::RepeatedIteration => "repeated_iteration",
            SmellKind::ShotgunSurgeryHazard => "shotgun_surgery_hazard",
            SmellKind::SiblingHelperRedefinition => "sibling_helper_redefinition",
            SmellKind::SplitVariable => "split_variable",
            SmellKind::SortInLoop => "sort_in_loop",
            SmellKind::SwallowedErrorSentinel => "swallowed_error_sentinel",
            SmellKind::TooManyReturns => "too_many_returns",
            SmellKind::TuplePacking => "tuple_packing",
            SmellKind::UnnecessaryNestedIf => "unnecessary_nested_if",
            SmellKind::WeakTestOracle => "weak_test_oracle",
        }
    }
}

impl std::fmt::Display for SmellKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
