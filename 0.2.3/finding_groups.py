"""High-level grouping for generated finding docs."""

from __future__ import annotations

from collections.abc import Iterable, Iterator

from finding_types import FindingGroup, FindingGroupTitle, Smell, SmellTerm


GROUPS: tuple[FindingGroup, ...] = (
    (
        FindingGroupTitle.API_CLARITY,
        (
            SmellTerm.BOOLEAN_BLINDNESS,
            SmellTerm.LITERAL_MEMBERSHIP,
            SmellTerm.MAGIC_STRING_DEFAULT,
            SmellTerm.MAGIC_NUMBERS,
        ),
    ),
    (
        FindingGroupTitle.COMPLEXITY,
        (
            SmellTerm.DEEP_NESTING,
            SmellTerm.HIGH_COGNITIVE_COMPLEXITY,
            SmellTerm.HIGH_COMPLEXITY,
            SmellTerm.TOO_MANY_RETURNS,
            SmellTerm.UNNECESSARY_NESTED_IF,
        ),
    ),
    (
        FindingGroupTitle.RESPONSIBILITY,
        (
            SmellTerm.GOD_MODULE,
            SmellTerm.HEAVY_NESTED_FUNCTION,
            SmellTerm.LARGE_CLASS,
            SmellTerm.LONG_FUNCTION,
            SmellTerm.LONG_PARAMETER_LIST,
            SmellTerm.NARRATING_CODE,
        ),
    ),
    (
        FindingGroupTitle.COUPLING,
        (
            SmellTerm.DATA_CLUMP,
            SmellTerm.DIVERGENT_CHANGE,
            SmellTerm.FEATURE_ENVY,
            SmellTerm.INAPPROPRIATE_INTIMACY,
            SmellTerm.MESSAGE_CHAIN,
            SmellTerm.REFUSED_BEQUEST,
            SmellTerm.SHOTGUN_SURGERY_HAZARD,
        ),
    ),
    (
        FindingGroupTitle.DATA_MODELING,
        (
            SmellTerm.IMPLICIT_SCHEMA,
            SmellTerm.LOOSE_TYPING,
            SmellTerm.TUPLE_PACKING,
        ),
    ),
    (
        FindingGroupTitle.MUTATION,
        (
            SmellTerm.MUTATED_PARAMETER,
            SmellTerm.REASSIGNED_PARAMETER,
            SmellTerm.SPLIT_VARIABLE,
        ),
    ),
    (
        FindingGroupTitle.PERFORMANCE,
        (
            SmellTerm.NESTED_LOOP,
            SmellTerm.N_PLUS_ONE_CALL,
            SmellTerm.REPEATED_ITERATION,
            SmellTerm.SORT_IN_LOOP,
        ),
    ),
)


def grouped_smells(
    smells: Iterable[Smell],
) -> Iterator[tuple[FindingGroupTitle, list[Smell]]]:
    remaining = {smell.term: smell for smell in smells}
    for title, terms in GROUPS:
        group = [remaining.pop(term) for term in terms if term in remaining]
        if group:
            yield title, group
    if remaining:
        yield FindingGroupTitle.OTHER, list(remaining.values())
