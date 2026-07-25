"""Shared types for generated finding docs."""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum


class FindingGroupTitle(StrEnum):
    API_CLARITY = "API Clarity and Constants"
    COMPLEXITY = "Complexity and Control Flow"
    RESPONSIBILITY = "Size and Responsibility"
    COUPLING = "Coupling and Cohesion"
    DATA_MODELING = "Type and Data Modeling"
    MUTATION = "Mutation and State"
    PERFORMANCE = "Performance"
    OTHER = "Other"


class SmellTerm(StrEnum):
    BOOLEAN_BLINDNESS = "boolean_blindness"
    DATA_CLUMP = "data_clump"
    DEEP_NESTING = "deep_nesting"
    DIVERGENT_CHANGE = "divergent_change"
    FEATURE_ENVY = "feature_envy"
    GOD_MODULE = "god_module"
    HEAVY_NESTED_FUNCTION = "heavy_nested_function"
    HIGH_COGNITIVE_COMPLEXITY = "high_cognitive_complexity"
    HIGH_COMPLEXITY = "high_complexity"
    IMPLICIT_SCHEMA = "implicit_schema"
    INAPPROPRIATE_INTIMACY = "inappropriate_intimacy"
    LARGE_CLASS = "large_class"
    LITERAL_MEMBERSHIP = "literal_membership"
    LONG_FUNCTION = "long_function"
    LONG_PARAMETER_LIST = "long_parameter_list"
    LOOSE_TYPING = "loose_typing"
    MAGIC_NUMBERS = "magic_numbers"
    MAGIC_STRING_DEFAULT = "magic_string_default"
    MESSAGE_CHAIN = "message_chain"
    MUTATED_PARAMETER = "mutated_parameter"
    NARRATING_CODE = "narrating_code"
    N_PLUS_ONE_CALL = "n_plus_one_call"
    NESTED_LOOP = "nested_loop"
    REASSIGNED_PARAMETER = "reassigned_parameter"
    REFUSED_BEQUEST = "refused_bequest"
    REPEATED_ITERATION = "repeated_iteration"
    SHOTGUN_SURGERY_HAZARD = "shotgun_surgery_hazard"
    SORT_IN_LOOP = "sort_in_loop"
    SPLIT_VARIABLE = "split_variable"
    TOO_MANY_RETURNS = "too_many_returns"
    TUPLE_PACKING = "tuple_packing"
    UNNECESSARY_NESTED_IF = "unnecessary_nested_if"


FindingGroup = tuple[FindingGroupTitle, tuple[SmellTerm, ...]]


@dataclass(frozen=True)
class Smell:
    kind: str
    term: SmellTerm
    title: str
    explanation: str
