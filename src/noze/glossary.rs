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

pub const ALL_SMELLS: [SmellKind; 31] = {
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
    ]
};

pub const SMELLS: [SmellDoc; 31] = {
    use SmellKind::*;
    [
        SmellDoc { kind: BooleanBlindness, title: "Boolean Blindness", explanation: "Bare booleans whose meaning is invisible at the call site (`f(True, False)`) — use an enum or keyword args so calls read clearly." },
        SmellDoc { kind: DataClump, title: "Data Clump", explanation: "The same group of values is passed together through many functions — bundle them into one object or typed structure." },
        SmellDoc { kind: DeepNesting, title: "Deep Nesting", explanation: "Control flow nests many levels deep, hard to follow — flatten with early returns or extracted helpers." },
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
        SmellDoc { kind: NestedLoop, title: "Nested Loop", explanation: "A loop is nested directly or through a helper called inside a loop — work grows multiplicatively; combine passes or pre-index the data." },
        SmellDoc { kind: NPlusOneCall, title: "N+1 Loop Call", explanation: "An external-looking call runs once per loop item — prefer a bulk query/request or prefetch so work scales by batch, not item." },
        SmellDoc { kind: ReassignedParameter, title: "Reassigned Parameter", explanation: "A parameter is rebound to a new value inside the body — confusing; use a separate local." },
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
pub mod docs {
    #![allow(dead_code)]
    // This module is docs-only metadata: the Python docs generator reads it
    // from source text, so the binary never constructs/inspects most fields;
    // silence "field never read" so the gate stays clean.

    use super::SmellKind;
    use SmellKind::*;

    pub struct ExternalLint {
        pub tool: &'static str,
        pub rule: &'static str,
    }

    #[derive(Clone, Copy)]
    pub struct ReferenceLink {
        pub label: &'static str,
        pub url: &'static str,
    }

    pub struct LanguageBlock {
        pub language: &'static str,
        pub body: &'static str,
    }

    pub struct FindingDocs {
        pub kind: crate::report::SmellKind,
        pub why_bad: &'static str,
        pub external_lints: &'static [ExternalLint],
        pub references: &'static [ReferenceLink],
        pub fixes: &'static [LanguageBlock],
    }

    pub const RG_DATA_CLUMPS: ReferenceLink = ReferenceLink {
        label: "Refactoring.Guru: Data Clumps",
        url: "https://refactoring.guru/smells/data-clumps",
    };
    pub const RG_DIVERGENT_CHANGE: ReferenceLink = ReferenceLink {
        label: "Refactoring.Guru: Divergent Change",
        url: "https://refactoring.guru/smells/divergent-change",
    };
    pub const RG_FEATURE_ENVY: ReferenceLink = ReferenceLink {
        label: "Refactoring.Guru: Feature Envy",
        url: "https://refactoring.guru/smells/feature-envy",
    };
    pub const RG_INAPPROPRIATE_INTIMACY: ReferenceLink = ReferenceLink {
        label: "Refactoring.Guru: Inappropriate Intimacy",
        url: "https://refactoring.guru/smells/inappropriate-intimacy",
    };
    pub const RG_LARGE_CLASS: ReferenceLink = ReferenceLink {
        label: "Refactoring.Guru: Large Class",
        url: "https://refactoring.guru/smells/large-class",
    };
    pub const RG_LONG_METHOD: ReferenceLink = ReferenceLink {
        label: "Refactoring.Guru: Long Method",
        url: "https://refactoring.guru/smells/long-method",
    };
    pub const RG_LONG_PARAMETER_LIST: ReferenceLink = ReferenceLink {
        label: "Refactoring.Guru: Long Parameter List",
        url: "https://refactoring.guru/smells/long-parameter-list",
    };
    pub const RG_MESSAGE_CHAINS: ReferenceLink = ReferenceLink {
        label: "Refactoring.Guru: Message Chains",
        url: "https://refactoring.guru/smells/message-chains",
    };
    pub const RG_REFUSED_BEQUEST: ReferenceLink = ReferenceLink {
        label: "Refactoring.Guru: Refused Bequest",
        url: "https://refactoring.guru/smells/refused-bequest",
    };
    pub const RG_SHOTGUN_SURGERY: ReferenceLink = ReferenceLink {
        label: "Refactoring.Guru: Shotgun Surgery",
        url: "https://refactoring.guru/smells/shotgun-surgery",
    };

    #[rustfmt::skip]
    pub const FINDINGS: &[FindingDocs] = &[
FindingDocs {
            kind: BooleanBlindness,
            why_bad: "Call sites stop reading like code and start reading like truth tables.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Promote each boolean decision into a named strategy so callers choose behavior explicitly.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Promote each boolean decision into a named strategy so callers choose behavior explicitly.",
                },
            ],
        },
        FindingDocs {
            kind: DataClump,
            why_bad: "The same bundle has to be kept in sync everywhere it travels.",
            external_lints: &[
            ],
            references: &[
                RG_DATA_CLUMPS,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Introduce a dataclass, TypedDict, or domain object for the repeated fields.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Introduce an interface or value object and pass that object through the API.",
                },
            ],
        },
        FindingDocs {
            kind: DeepNesting,
            why_bad: "The control flow becomes hard to scan and easy to misread in review.",
            external_lints: &[
                ExternalLint { tool: "ruff", rule: "PLR1702" },
                ExternalLint { tool: "eslint", rule: "max-depth" },
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Use guard clauses, continue early, or extract a helper for the nested branch.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Flatten with early returns/continues or pull nested checks into named helpers.",
                },
            ],
        },
        FindingDocs {
            kind: DivergentChange,
            why_bad: "One file starts changing for unrelated reasons, so fixes get tangled.",
            external_lints: &[
            ],
            references: &[
                RG_DIVERGENT_CHANGE,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Split the module by reason-to-change: presentation, pricing, persistence, etc.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Move unrelated responsibilities into focused modules or services.",
                },
            ],
        },
        FindingDocs {
            kind: FeatureEnvy,
            why_bad: "Logic sits next to the wrong data, so edits keep reaching through a foreign object.",
            external_lints: &[
            ],
            references: &[
                RG_FEATURE_ENVY,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Move the behavior onto the object that owns most of the data.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Put the calculation on the owning class/module or expose a narrow query method.",
                },
            ],
        },
        FindingDocs {
            kind: GodModule,
            why_bad: "A single module becomes a hot spot with too many reasons to change.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Split by dependency direction and cohesive responsibility.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Extract smaller modules and route callers through a narrow public API.",
                },
            ],
        },
        FindingDocs {
            kind: HeavyNestedFunction,
            why_bad: "Nested helpers hide important behavior and make tests awkward.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Promote the helper to a top-level private function with direct tests.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Move the nested function to module scope or a small collaborator.",
                },
            ],
        },
        FindingDocs {
            kind: HighCognitiveComplexity,
            why_bad: "The reader has to simulate too many branches and nesting levels at once.",
            external_lints: &[
                ExternalLint { tool: "eslint", rule: "sonarjs/cognitive-complexity" },
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Name the decision steps, flatten branches, and extract cohesive helpers.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Prefer guard clauses and small predicate functions over nested branches.",
                },
            ],
        },
FindingDocs {
            kind: HighComplexity,
            why_bad: "The function accumulates too many distinct paths to reason about confidently.",
            external_lints: &[
                ExternalLint { tool: "ruff", rule: "PLR0912" },
                ExternalLint { tool: "eslint", rule: "complexity" },
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Use a dispatch table or split the branches into named operations.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Use a map, strategy object, or smaller functions for independent paths.",
                },
            ],
        },
        FindingDocs {
            kind: ImplicitSchema,
            why_bad: "Stringly typed payloads drift silently when the shape changes.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Model the payload with a dataclass, TypedDict, or Pydantic model.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Replace loose records with an interface or validated schema.",
                },
            ],
        },
        FindingDocs {
            kind: LargeClass,
            why_bad: "The class stops having a clear job and becomes a grab bag.",
            external_lints: &[
            ],
            references: &[
                RG_LARGE_CLASS,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Extract cohesive collaborators around loading, rendering, delivery, etc.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Split the class by capability and keep a thin orchestration surface.",
                },
            ],
        },
        FindingDocs {
            kind: InappropriateIntimacy,
            why_bad: "Two classes know too much about each other's internals, so refactors ripple.",
            external_lints: &[
            ],
            references: &[
                RG_INAPPROPRIATE_INTIMACY,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Add a public method on the collaborator or merge the coupled objects.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Expose a narrow method/property instead of reaching into internals.",
                },
            ],
        },
        FindingDocs {
            kind: LiteralMembership,
            why_bad: "Hard-coded string sets become a hidden enum that tools cannot help with.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Use an Enum or named constant set with a typed boundary.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Use a union type, enum, or const object with a derived type.",
                },
            ],
        },
        FindingDocs {
            kind: LongFunction,
            why_bad: "The function becomes a scroll instead of a unit you can hold in your head.",
            external_lints: &[
                ExternalLint { tool: "ruff", rule: "PLR0915" },
                ExternalLint { tool: "eslint", rule: "max-lines-per-function" },
            ],
            references: &[
                RG_LONG_METHOD,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Extract named chunks that each complete one step of the workflow.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Extract cohesive helper functions and keep the orchestration readable.",
                },
            ],
        },
        FindingDocs {
            kind: LongParameterList,
            why_bad: "The call contract becomes noisy and easy to pass in the wrong order.",
            external_lints: &[
                ExternalLint { tool: "ruff", rule: "PLR0913" },
                ExternalLint { tool: "eslint", rule: "max-params" },
            ],
            references: &[
                RG_LONG_PARAMETER_LIST,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Group related parameters into a dataclass or keyword-only options object.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Use an options object or domain type instead of positional arguments.",
                },
            ],
        },
        FindingDocs {
            kind: LooseTyping,
            why_bad: "Weak types make invalid states look valid until runtime.",
            external_lints: &[],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Use a dataclass or another concrete model so callers pass named fields instead of loose keys.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Replace any with an interface, unknown plus narrowing, or a schema-derived type.",
                },
            ],
        },
        FindingDocs {
            kind: MagicStringDefault,
            why_bad: "A sentinel string hides the real optionality of the value.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Reject a missing required string explicitly instead of hiding it behind a sentinel fallback.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Throw on a missing required string, or model optionality explicitly when absence is valid.",
                },
            ],
        },
FindingDocs {
            kind: MagicNumbers,
            why_bad: "Numbers without names are hard to audit and easy to copy blindly.",
            external_lints: &[
                ExternalLint { tool: "ruff", rule: "PLR2004" },
                ExternalLint { tool: "eslint", rule: "no-magic-numbers" },
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Extract a named constant near the policy it represents.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Use a named const or configuration value for the policy number.",
                },
            ],
        },
        FindingDocs {
            kind: MessageChain,
            why_bad: "Deep property walking couples the caller to the whole object graph.",
            external_lints: &[
            ],
            references: &[
                RG_MESSAGE_CHAINS,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Ask the nearest object for the answer through a method or property.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Expose a query on the immediate collaborator instead of chaining internals.",
                },
            ],
        },
        FindingDocs {
            kind: MutatedParameter,
            why_bad: "Mutating inputs hides side effects and makes call order matter.",
            external_lints: &[
                ExternalLint { tool: "eslint", rule: "no-param-reassign" },
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Return a new collection or make the mutation explicit in the API name.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Return a copied value or make mutation an intentional method on an owner.",
                },
            ],
        },
        FindingDocs {
            kind: NestedLoop,
            why_bad: "Costs grow faster than the data and the code gets hard to flatten.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Pre-index one side, combine passes, or use a clearer iterator pipeline.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Build a lookup map or flatten the data before iterating.",
                },
            ],
        },
        FindingDocs {
            kind: NPlusOneCall,
            why_bad: "A per-item call can explode runtime and hit the backend one request at a time.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Batch-load or prefetch the related data before the loop.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Use a bulk endpoint/query or prefetch into a map before rendering.",
                },
            ],
        },
        FindingDocs {
            kind: ReassignedParameter,
            why_bad: "Rebinding a parameter muddies the original meaning of the value.",
            external_lints: &[
                ExternalLint { tool: "eslint", rule: "no-param-reassign" },
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Introduce a local variable for the transformed value.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Use a separate const for each semantic step.",
                },
            ],
        },
        FindingDocs {
            kind: RefusedBequest,
            why_bad: "Inheritance promises behavior the subclass does not actually want.",
            external_lints: &[
            ],
            references: &[
                RG_REFUSED_BEQUEST,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Prefer composition or split the base class into smaller capabilities.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Use composition or narrower interfaces instead of a broad base class.",
                },
            ],
        },
        FindingDocs {
            kind: RepeatedIteration,
            why_bad: "The same collection gets scanned over and over when one pass would do.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Fuse compatible passes or cache the intermediate result.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Combine loops when the operations share the same traversal.",
                },
            ],
        },
FindingDocs {
            kind: ShotgunSurgeryHazard,
            why_bad: "One edit fans out to many dependents.",
            external_lints: &[
            ],
            references: &[
                RG_SHOTGUN_SURGERY,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Separate stable interfaces from volatile implementation details.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Reduce fan-in by splitting policy from shared utility shape.",
                },
            ],
        },
        FindingDocs {
            kind: SplitVariable,
            why_bad: "A variable with multiple meanings is a trap for both readers and debuggers.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Use separate locals named for each meaning.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Prefer distinct const bindings for distinct concepts.",
                },
            ],
        },
        FindingDocs {
            kind: SortInLoop,
            why_bad: "Repeated sorts turn a small loop into a surprisingly expensive one.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Sort once before the loop or keep data ordered as it is built.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Hoist sorting out of the loop or maintain an ordered structure.",
                },
            ],
        },
        FindingDocs {
            kind: TooManyReturns,
            why_bad: "Many exit points make the function harder to follow and test.",
            external_lints: &[
                ExternalLint { tool: "ruff", rule: "PLR0911" },
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Group related guards or extract decision helpers.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Keep meaningful guard clauses, then extract noisy branches into predicates.",
                },
            ],
        },
        FindingDocs {
            kind: TuplePacking,
            why_bad: "Positional bundles hide meaning and make the code brittle to reorderings.",
            external_lints: &[
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Use a named tuple, dataclass, or object with explicit fields.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Use an interface/object instead of anonymous positional tuple data.",
                },
            ],
        },
        FindingDocs {
            kind: UnnecessaryNestedIf,
            why_bad: "The code says 'if' twice when the condition really belongs on one line.",
            external_lints: &[
                ExternalLint { tool: "eslint", rule: "sonarjs/no-collapsible-if" },
            ],
            references: &[],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Combine the conditions or use a guard clause.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Collapse nested conditions with && or extract a predicate.",
                },
            ],
        },
    ];

    pub fn all() -> impl Iterator<Item = &'static FindingDocs> {
        FINDINGS.iter()
    }
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
            assert!(!doc.references.is_empty());
            assert!(doc.fixes.iter().any(|block| block.language == "python"));
            assert!(doc.fixes.iter().any(|block| block.language == "typescript"));
        }
    }
}
