//! Plain-English definitions for every noze finding category. These feed terminal
//! `--explain`, the `explain` CLI subcommand, generated docs, and MCP explain.

pub struct GlossaryDoc {
    pub term: &'static str,
    pub title: &'static str,
    pub explanation: &'static str,
}

pub struct SmellDoc {
    pub kind: SmellKind,
    pub title: &'static str,
    pub explanation: &'static str,
}

pub const PILLARS: [GlossaryDoc; 5] = [
    GlossaryDoc { term: "duplication", title: "Duplication", explanation: "Structurally identical code appears in several places — extract a shared function so a change or fix happens once, not N times." },
    GlossaryDoc { term: "dead_code", title: "Dead Code", explanation: "A symbol nothing reachable references — likely safe to delete (the confidence tier says how sure Sensez is, given dynamic use can hide a caller)." },
    GlossaryDoc { term: "cycles", title: "Import Cycle", explanation: "Modules that import each other in a loop — brittle to change and load-order dependent; break it by extracting the shared piece or inverting one dependency." },
    GlossaryDoc { term: "boundaries", title: "Boundary Violation", explanation: "An import crosses an architectural rule you configured (e.g. core importing api) — keep layered dependencies pointing one way." },
    GlossaryDoc { term: "smells", title: "Design Smell", explanation: "A structural maintainability issue in a function or class (complexity, coupling, cohesion, typing) that makes the code harder to change safely." },
];

pub const ALL_SMELLS: [SmellKind; 36] = {
    use SmellKind::*;
    [
        BooleanBlindness,
        DataClump,
        DeepNesting,
        DefensiveFallback,
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
        MutatedParameter,
        NarratingCode,
        NestedLoop,
        NestedTernary,
        NPlusOneCall,
        ReassignedParameter,
        RedundantValidation,
        RefusedBequest,
        RepeatedIteration,
        ShotgunSurgeryHazard,
        SplitVariable,
        SortInLoop,
        TooManyReturns,
        TuplePacking,
        UnnecessaryNestedIf,
    ]
};

pub const SMELLS: [SmellDoc; 36] = {
    use SmellKind::*;
    [
        SmellDoc { kind: BooleanBlindness, title: "Boolean Blindness", explanation: "Bare booleans whose meaning is invisible at the call site (`f(True, False)`) — use an enum or keyword args so calls read clearly." },
        SmellDoc { kind: DataClump, title: "Data Clump", explanation: "The same group of values is passed together through many functions — bundle them into one object or typed structure." },
        SmellDoc { kind: DeepNesting, title: "Deep Nesting", explanation: "Control flow nests many levels deep, hard to follow — flatten with early returns or extracted helpers." },
        SmellDoc { kind: DefensiveFallback, title: "Defensive Fallback Soup", explanation: "Broad error handling and repeated empty defaults hide invalid states — validate the boundary and let unexpected failures remain visible." },
        SmellDoc { kind: DivergentAbstraction, title: "Divergent Abstraction", explanation: "An abstraction has only two implementations that grow in different directions — remove the forced common type or narrow it to genuine shared behavior." },
        SmellDoc { kind: DivergentChange, title: "Divergent Change", explanation: "One module gets edited for many unrelated reasons — it has too many responsibilities; split it along those axes." },
        SmellDoc { kind: FeatureEnvy, title: "Feature Envy", explanation: "A method uses another object's data more than its own — move it onto the class that owns that data." },
        SmellDoc { kind: GodModule, title: "God Module", explanation: "A module that too much of the codebase depends on (high centrality) — a coupling and change-risk hotspot; split its responsibilities." },
        SmellDoc { kind: HeavyNestedFunction, title: "Heavy Nested Function", explanation: "An inner/nested function that grew large and logic-heavy — promote it to a top-level, testable function." },
        SmellDoc { kind: HighCognitiveComplexity, title: "High Cognitive Complexity", explanation: "Hard for a human to follow — nested branches and loops weighted by depth; simplify or break it up." },
        SmellDoc { kind: HighComplexity, title: "High Cyclomatic Complexity", explanation: "Many independent paths through the function, so it's hard to test fully — decompose it." },
        SmellDoc { kind: ImplicitSchema, title: "Implicit Schema", explanation: "A dict/object accessed by many string keys — an unwritten schema; model it as a typed structure." },
        SmellDoc { kind: InappropriateIntimacy, title: "Inappropriate Intimacy", explanation: "Two classes each reach into the other's internals, so neither can change independently — narrow the shared surface or merge them." },
        SmellDoc { kind: LargeClass, title: "Large Class", explanation: "A class with too many methods/responsibilities — split it into focused classes." },
        SmellDoc { kind: LiteralMembership, title: "Literal Membership", explanation: "Branching on membership in a literal string list (`x in ['a','b']`) — stringly-typed categories; use an Enum." },
        SmellDoc { kind: LongFunction, title: "Long Function", explanation: "Too many lines to grasp at once — extract cohesive pieces." },
        SmellDoc { kind: LongParameterList, title: "Long Parameter List", explanation: "Too many parameters — group related ones into an object, or the function is doing too much." },
        SmellDoc { kind: LooseTyping, title: "Loose Typing", explanation: "A public signature leans on vague types (`Any`/untyped/overly broad) — tighten annotations so callers and tools know the contract." },
        SmellDoc { kind: MagicStringDefault, title: "Magic String Default", explanation: "A fallback empty or one-character string is standing in for an optional/nullable value (`or \"\"`, `|| \"?\"`) — the contract is hiding in a sentinel; prefer a nullable/optional string or a dedicated sum type." },
        SmellDoc { kind: MagicNumbers, title: "Magic Numbers", explanation: "Unexplained numeric literals in logic — name them as constants so their intent is clear." },
        SmellDoc { kind: MessageChain, title: "Message Chain", explanation: "A long `a.b.c.d` access chain couples the caller to a deep object graph (Law of Demeter) — ask the immediate collaborator instead." },
        SmellDoc { kind: MutatedParameter, title: "Mutated Parameter", explanation: "The function mutates a caller's argument in place — a hidden side effect; return a new value instead." },
        SmellDoc { kind: NarratingCode, title: "Narrating Code", explanation: "A function is packed with explanatory comments — prefer clearer names or extracted helpers, keeping comments for why." },
        SmellDoc { kind: NestedLoop, title: "Nested Loop", explanation: "A loop is nested directly or through a helper called inside a loop — work grows multiplicatively; combine passes or pre-index the data." },
        SmellDoc { kind: NestedTernary, title: "Nested Ternary", explanation: "A conditional expression contains another conditional expression, forcing readers to match several branches mentally — extract the result into a named function or flatten it into if statements with early returns." },
        SmellDoc { kind: NPlusOneCall, title: "N+1 Loop Call", explanation: "An external-looking call runs once per loop item — prefer a bulk query/request or prefetch so work scales by batch, not item." },
        SmellDoc { kind: ReassignedParameter, title: "Reassigned Parameter", explanation: "A parameter is rebound to a new value inside the body — confusing; use a separate local." },
        SmellDoc { kind: RedundantValidation, title: "Redundant Validation", explanation: "The same condition is checked repeatedly in one function — establish the invariant once and simplify the later path." },
        SmellDoc { kind: RefusedBequest, title: "Refused Bequest", explanation: "A subclass inherits methods/fields it doesn't use or stubs out — the inheritance is wrong; prefer composition." },
        SmellDoc { kind: RepeatedIteration, title: "Repeated Iteration", explanation: "The same collection is iterated several times in one scope — fuse the passes so the data is scanned once." },
        SmellDoc { kind: ShotgunSurgeryHazard, title: "Shotgun Surgery Hazard", explanation: "A symbol so widely depended-on that one change ripples across many modules — a blast-radius hotspot." },
        SmellDoc { kind: SplitVariable, title: "Split Variable", explanation: "One local is reassigned to mean different things at different points — use distinct, single-purpose bindings." },
        SmellDoc { kind: SortInLoop, title: "Sort In Loop", explanation: "A collection is sorted inside a loop — hoist sorting or maintain ordered data to avoid repeated O(n log n) work." },
        SmellDoc { kind: TooManyReturns, title: "Too Many Returns", explanation: "Many exit points make the function's flow hard to follow — consolidate, or it's doing too much." },
        SmellDoc { kind: TuplePacking, title: "Tuple Packing", explanation: "Data passed as positional tuples whose fields aren't named — use a named structure so meaning is explicit." },
        SmellDoc { kind: UnnecessaryNestedIf, title: "Unnecessary Nested If", explanation: "An `if` whose only body is another `if`, with no else path — combine the conditions with `and`/`&&` to flatten the control flow." },
    ]
};

pub fn smell(kind: SmellKind) -> &'static SmellDoc {
    match SMELLS.iter().find(|doc| doc.kind == kind) {
        Some(doc) => doc,
        None => unreachable!("all smell kinds are documented"),
    }
}

use crate::report::{AnalysisReport, GlossaryEntry, SmellKind};

impl SmellKind {
    /// Human label, e.g. "Inappropriate Intimacy".
    pub fn title(self) -> &'static str {
        smell(self).title
    }

    /// One sentence: what it is, why it matters, and the fix nudge.
    pub fn explanation(self) -> &'static str {
        smell(self).explanation
    }
}

fn entry(term: &str, title: &str, explanation: &str) -> GlossaryEntry {
    GlossaryEntry {
        term: term.to_string(),
        title: title.to_string(),
        explanation: explanation.to_string(),
    }
}

fn smell_entry(kind: SmellKind) -> GlossaryEntry {
    let doc = smell(kind);
    entry(kind.as_str(), doc.title, doc.explanation)
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

/// Look up any term: a pillar key or a smell kind string.
pub fn lookup(term: &str) -> Option<GlossaryEntry> {
    if let Some(pillar) = PILLARS.iter().find(|entry| entry.term == term) {
        return Some(entry(pillar.term, pillar.title, pillar.explanation));
    }
    ALL_SMELLS
        .iter()
        .find(|kind| kind.as_str() == term)
        .map(|kind| smell_entry(*kind))
}

/// Every definition (pillars + all smell kinds), for `explain` with no term.
pub fn all() -> Vec<GlossaryEntry> {
    PILLARS
        .iter()
        .map(|doc| entry(doc.term, doc.title, doc.explanation))
        .chain(ALL_SMELLS.iter().map(|kind| smell_entry(*kind)))
        .collect()
}

#[cfg(feature = "docs")]
pub mod docs;
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
        let cycles = match lookup("cycles") {
            Some(entry) => entry,
            None => panic!("cycles should resolve"),
        };
        assert_eq!(cycles.title, "Import Cycle");
        assert!(lookup("not_a_thing").is_none());
    }

    #[cfg(feature = "docs")]
    #[test]
    fn docs_metadata_covers_every_smell_kind() {
        for kind in ALL_SMELLS {
            let doc = docs::all().find(|doc| doc.kind == kind);
            assert!(doc.is_some(), "missing docs metadata for {}", kind.as_str());
            let doc = match doc {
                Some(doc) => doc,
                None => continue,
            };
            assert!(!doc.why_bad.is_empty());
            assert!(doc.fixes.iter().any(|block| block.language == "python"));
            assert!(doc.fixes.iter().any(|block| block.language == "typescript"));
        }
    }
}
