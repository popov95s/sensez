//! Plain-English definitions for every finding category — the single source of
//! truth for `meta.glossary` (auto-attached, present categories only), the
//! `explain` CLI subcommand, and the MCP `explain` tool. Authored to match what
//! sensez actually measures, so explanations never drift from the heuristics.

use crate::report::{AnalysisReport, GlossaryEntry, SmellKind};

/// The five pillars: `(key, title, explanation)`.
const PILLARS: [(&str, &str, &str); 5] = [
    (
        "duplication",
        "Duplication",
        "Structurally identical code appears in several places — extract a shared \
         function so a change or fix happens once, not N times.",
    ),
    (
        "dead_code",
        "Dead Code",
        "A symbol nothing reachable references — likely safe to delete (the \
         confidence tier says how sure Sensez is, given dynamic use can hide a caller).",
    ),
    (
        "cycles",
        "Import Cycle",
        "Modules that import each other in a loop — brittle to change and \
         load-order dependent; break it by extracting the shared piece or \
         inverting one dependency.",
    ),
    (
        "boundaries",
        "Boundary Violation",
        "An import crosses an architectural rule you configured (e.g. core \
         importing api) — keep layered dependencies pointing one way.",
    ),
    (
        "smells",
        "Design Smell",
        "A structural maintainability issue in a function or class (complexity, \
         coupling, cohesion, typing) that makes the code harder to change safely.",
    ),
];

impl SmellKind {
    /// Human label, e.g. "Inappropriate Intimacy".
    pub fn title(self) -> &'static str {
        self.gloss().0
    }

    /// One sentence: what it is, why it matters, and the fix nudge.
    pub fn explanation(self) -> &'static str {
        self.gloss().1
    }

    fn gloss(self) -> (&'static str, &'static str) {
        use SmellKind::*;
        match self {
            BooleanBlindness => ("Boolean Blindness", "Bare booleans whose meaning is invisible at the call site (`f(True, False)`) — use an enum or keyword args so calls read clearly."),
            DataClump => ("Data Clump", "The same group of values is passed together through many functions — bundle them into one object/dataclass."),
            DeepNesting => ("Deep Nesting", "Control flow nests many levels deep, hard to follow — flatten with early returns or extracted helpers."),
            DivergentChange => ("Divergent Change", "One module gets edited for many unrelated reasons — it has too many responsibilities; split it along those axes."),
            FeatureEnvy => ("Feature Envy", "A method uses another object's data more than its own — move it onto the class that owns that data."),
            GodModule => ("God Module", "A module that too much of the codebase depends on (high centrality) — a coupling and change-risk hotspot; split its responsibilities."),
            HeavyNestedFunction => ("Heavy Nested Function", "An inner/nested function that grew large and logic-heavy — promote it to a top-level, testable function."),
            HighCognitiveComplexity => ("High Cognitive Complexity", "Hard for a human to follow — nested branches and loops weighted by depth; simplify or break it up."),
            HighComplexity => ("High Cyclomatic Complexity", "Many independent paths through the function, so it's hard to test fully — decompose it."),
            ImplicitSchema => ("Implicit Schema", "A dict/object accessed by many string keys — an unwritten schema; model it as a dataclass/typed structure."),
            InappropriateIntimacy => ("Inappropriate Intimacy", "Two classes each reach into the other's internals, so neither can change independently — narrow the shared surface or merge them."),
            LargeClass => ("Large Class", "A class with too many methods/responsibilities — split it into focused classes."),
            LiteralMembership => ("Literal Membership", "Branching on membership in a literal string list (`x in ['a','b']`) — stringly-typed categories; use an Enum."),
            LongFunction => ("Long Function", "Too many lines to grasp at once — extract cohesive pieces."),
            LongParameterList => ("Long Parameter List", "Too many parameters — group related ones into an object, or the function is doing too much."),
            LooseTyping => ("Loose Typing", "A public signature leans on vague types (`Any`/untyped/overly broad) — tighten annotations so callers and tools know the contract."),
            MagicNumbers => ("Magic Numbers", "Unexplained numeric literals in logic — name them as constants so their intent is clear."),
            MessageChain => ("Message Chain", "A long `a.b.c.d` access chain couples the caller to a deep object graph (Law of Demeter) — ask the immediate collaborator instead."),
            MutatedParameter => ("Mutated Parameter", "The function mutates a caller's argument in place — a hidden side effect; return a new value instead."),
            ReassignedParameter => ("Reassigned Parameter", "A parameter is rebound to a new value inside the body — confusing; use a separate local."),
            RefusedBequest => ("Refused Bequest", "A subclass inherits methods/fields it doesn't use or stubs out — the inheritance is wrong; prefer composition."),
            ShotgunSurgeryHazard => ("Shotgun Surgery Hazard", "A symbol so widely depended-on that one change ripples across many modules — a blast-radius hotspot."),
            SplitVariable => ("Split Variable", "One local is reassigned to mean different things at different points — use distinct, single-purpose bindings."),
            TooManyReturns => ("Too Many Returns", "Many exit points make the function's flow hard to follow — consolidate, or it's doing too much."),
            TuplePacking => ("Tuple Packing", "Data passed as positional tuples whose fields aren't named — use a named structure so meaning is explicit."),
        }
    }
}

/// All smell kinds, for `explain` (no arg) and exhaustiveness in tests.
pub const ALL_SMELLS: [SmellKind; 25] = {
    use SmellKind::*;
    [
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
        MagicNumbers,
        MessageChain,
        MutatedParameter,
        ReassignedParameter,
        RefusedBequest,
        ShotgunSurgeryHazard,
        SplitVariable,
        TooManyReturns,
        TuplePacking,
    ]
};

fn entry(term: &str, title: &str, explanation: &str) -> GlossaryEntry {
    GlossaryEntry {
        term: term.to_string(),
        title: title.to_string(),
        explanation: explanation.to_string(),
    }
}

fn smell_entry(kind: SmellKind) -> GlossaryEntry {
    entry(kind.as_str(), kind.title(), kind.explanation())
}

/// Definitions for exactly the categories present in `report` (deduped): each
/// non-empty pillar, then each distinct smell kind that appears.
pub fn for_report(report: &AnalysisReport) -> Vec<GlossaryEntry> {
    let mut out = Vec::new();
    let present = [
        ("duplication", !report.duplication.is_empty()),
        ("dead_code", !report.dead_code.is_empty()),
        ("cycles", !report.cycles.is_empty()),
        ("boundaries", !report.boundaries.is_empty()),
    ];
    for (key, here) in present {
        if here {
            if let Some(e) = lookup(key) {
                out.push(e);
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for smell in &report.smells {
        if seen.insert(smell.kind.as_str()) {
            out.push(smell_entry(smell.kind));
        }
    }
    out
}

/// Look up any term — a pillar key or a smell kind string — for `explain`.
pub fn lookup(term: &str) -> Option<GlossaryEntry> {
    if let Some((key, title, ex)) = PILLARS.iter().find(|(k, ..)| *k == term) {
        return Some(entry(key, title, ex));
    }
    ALL_SMELLS
        .iter()
        .find(|k| k.as_str() == term)
        .map(|k| smell_entry(*k))
}

/// Every definition (pillars + all smell kinds), for `explain` with no term.
pub fn all() -> Vec<GlossaryEntry> {
    PILLARS
        .iter()
        .map(|(k, t, e)| entry(k, t, e))
        .chain(ALL_SMELLS.iter().map(|k| smell_entry(*k)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_smell_kind_has_a_distinct_title_and_explanation() {
        let titles: std::collections::BTreeSet<_> = ALL_SMELLS.iter().map(|k| k.title()).collect();
        assert_eq!(titles.len(), ALL_SMELLS.len(), "titles must be unique");
        for kind in ALL_SMELLS {
            assert!(!kind.explanation().is_empty());
            assert!(
                lookup(kind.as_str()).is_some(),
                "{} looks up",
                kind.as_str()
            );
        }
    }

    #[test]
    fn lookup_resolves_pillars_and_unknown_is_none() {
        assert_eq!(lookup("cycles").unwrap().title, "Import Cycle");
        assert!(lookup("not_a_thing").is_none());
    }
}
