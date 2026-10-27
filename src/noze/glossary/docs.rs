//! Docs-only smell metadata is intentionally kept in one file.
//!
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
pub const RG_COMMENTS: ReferenceLink = ReferenceLink {
    label: "Refactoring.Guru: Comments",
    url: "https://refactoring.guru/smells/comments",
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
                    body: "Use a string-literal union type or a named `as const` set shared with its type.",
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
                    body: "Use a dataclass or another concrete model so callers pass named fields instead of loose keys. Do not fix this by creating a shallow alias such as `UserPayload = dict[str, Any]` or `UserId = str`.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Replace any with an interface, unknown plus narrowing, or a schema-derived type. Type-guard subjects are never reported and bare `unknown` params only at high strictness. Do not fix this by creating a shallow alias such as `type UserPayload = Record<string, any>` or `type UserId = string`.",
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
            kind: NarratingCode,
            why_bad: "The prose becomes a second implementation that readers must keep in sync with the code.",
            external_lints: &[],
            references: &[
                RG_COMMENTS,
            ],
            fixes: &[
                LanguageBlock {
                    language: "python",
                    body: "Rename values and extract helper functions so the code says what the comments were saying.",
                },
                LanguageBlock {
                    language: "typescript",
                    body: "Extract named predicates/helpers and keep comments for constraints or rationale.",
                },
                LanguageBlock {
                    language: "rust",
                    body: "Move step-by-step narration into names and helper functions; keep comments for invariants and safety rationale.",
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
        external_lints: &[],
        references: &[RG_REFUSED_BEQUEST],
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
        kind: NestedTernary,
        why_bad: "Nested conditional expressions force readers to mentally match several conditions with their results, making agent-generated logic difficult to review and change.",
        external_lints: &[ExternalLint { tool: "eslint", rule: "no-nested-ternary" }],
        references: &[],
        fixes: &[
            LanguageBlock {
                language: "python",
                body: "Extract the decision into a named helper, or replace the expression with ordered if statements and early returns.",
            },
            LanguageBlock {
                language: "typescript",
                body: "Move result selection into a named function and use explicit if branches with early returns.",
            },
        ],
    },
    FindingDocs {
        kind: RepeatedIteration,
        why_bad: "The same collection gets scanned over and over when one pass would do.",
        external_lints: &[],
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
        external_lints: &[],
        references: &[RG_SHOTGUN_SURGERY],
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
        kind: SiblingHelperRedefinition,
        why_bad: "A fix lands in one copy and the others drift; a reviewer reading one file cannot know the helper already exists.",
        external_lints: &[],
        references: &[],
        fixes: &[
            LanguageBlock {
                language: "python",
                body: "Keep one definition in a shared module and import it where the copies were.",
            },
            LanguageBlock {
                language: "typescript",
                body: "Export one definition from a shared module and import it where the copies were.",
            },
        ],
    },
    FindingDocs {
        kind: SplitVariable,
        why_bad: "A variable with multiple meanings is a trap for both readers and debuggers.",
        external_lints: &[],
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
        external_lints: &[],
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
        kind: SwallowedErrorSentinel,
        why_bad: "A handler that turns any failure into `null`, an empty list or a log line hands callers a plausible value; the broken contract surfaces far from its cause, or never.",
        external_lints: &[],
        references: &[],
        fixes: &[
            LanguageBlock {
                language: "python",
                body: "Catch the specific exception you can handle, re-raise the rest with `raise ... from error`, or return a typed failure the caller must check.",
            },
            LanguageBlock {
                language: "typescript",
                body: "Rethrow, or return a typed failure (`{ ok: false, error }` or a `Result`) so the caller must handle it; keep a `tryX(): T | null` helper only where missing is the documented contract.",
            },
        ],
    },
    FindingDocs {
        kind: TooManyReturns,
        why_bad: "Many exit points make the function harder to follow and test.",
        external_lints: &[ExternalLint {
            tool: "ruff",
            rule: "PLR0911",
        }],
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
        external_lints: &[],
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
        external_lints: &[ExternalLint {
            tool: "eslint",
            rule: "sonarjs/no-collapsible-if",
        }],
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
    FindingDocs {
        kind: DefensiveFallback,
        why_bad: "Broad catches plus empty defaults turn contract violations into plausible but wrong data.",
        external_lints: &[],
        references: &[],
        fixes: &[
            LanguageBlock { language: "python", body: "Validate once at the boundary, catch specific failures, and preserve unexpected errors." },
            LanguageBlock { language: "typescript", body: "Parse once at the boundary, catch specific failures, and preserve unexpected errors." },
        ],
    },
    FindingDocs {
        kind: DependencyReimplementation,
        why_bad: "Each private copy of a stock utility is one more thing to test and maintain, and copies drift: one swaps the bounds, another drops the last chunk.",
        external_lints: &[],
        references: &[],
        fixes: &[
            LanguageBlock { language: "python", body: "Call the standard-library or declared-package function the message names. List in-house utility packages in `providers` so their functions count too." },
            LanguageBlock { language: "typescript", body: "Import the function the message names from the package `package.json` already lists. List in-house utility packages in `providers` so they count too." },
        ],
    },
    FindingDocs {
        kind: DivergentAbstraction,
        why_bad: "A forced common type couples two implementations whose real responsibilities no longer match.",
        external_lints: &[],
        references: &[],
        fixes: &[
            LanguageBlock { language: "python", body: "Narrow the Protocol to genuine shared behavior, or use the concrete types directly." },
            LanguageBlock { language: "typescript", body: "Narrow the interface to genuine shared behavior, or use the concrete types directly." },
        ],
    },
    FindingDocs {
        kind: RedundantValidation,
        why_bad: "Repeated guards obscure which invariants have already been established.",
        external_lints: &[],
        references: &[],
        fixes: &[
            LanguageBlock { language: "python", body: "Validate at the boundary or first use, then rely on the established invariant." },
            LanguageBlock { language: "typescript", body: "Validate at the boundary or first use, then rely on the established invariant." },
        ],
    },
    FindingDocs {
        kind: RedundantWrapperChain,
        why_bad: "An unchanged forwarding layer adds indirection without behavior and makes navigation harder.",
        external_lints: &[],
        references: &[],
        fixes: &[LanguageBlock { language: "python", body: "Remove pass-through functions, or raise `max_depth` when a deliberate boundary is worth keeping." }, LanguageBlock { language: "typescript", body: "Remove pass-through functions, or raise `max_depth` when a deliberate boundary is worth keeping." }],
    },
    FindingDocs {
        kind: StubShipped,
        why_bad: "Scaffolding written for later ships as a function that only fails, and the first user on that path hits the error at runtime. Unreferenced stubs are also reported as dead code. Keep deliberate scaffolding folders out with `[smells] exclude`.",
        external_lints: &[],
        references: &[],
        fixes: &[LanguageBlock { language: "python", body: "Implement the function, delete it with its callers, or declare it with `@abstractmethod` on an `ABC`/`Protocol`." }, LanguageBlock { language: "typescript", body: "Implement the function, delete it with its callers, or declare it `abstract` or as an interface member. Tune the thrown messages with `markers`." }],
    },
    FindingDocs {
        kind: FixtureEchoAssertion,
        why_bad: "An assertion that compares a value with itself passes whatever the code under test does, so the test reports coverage it does not have.",
        external_lints: &[],
        references: &[],
        fixes: &[LanguageBlock { language: "python", body: "Compare the result of the call under test with an independently written expected value." }, LanguageBlock { language: "typescript", body: "Pass the result of the call under test to `expect` and an independently written value to the matcher; for mocks, also assert on the value the subject returned." }],
    },
    FindingDocs {
        kind: WeakTestOracle,
        why_bad: "A test that only executes code can pass when behavior is wrong.",
        external_lints: &[],
        references: &[],
        fixes: &[LanguageBlock { language: "python", body: "Assert an observable result or expected exception." }, LanguageBlock { language: "typescript", body: "Assert the result, state, or expected rejection with a matcher." }],
    },
    FindingDocs {
        kind: MockedSubject,
        why_bad: "A test that replaces the module it claims to test checks the double's canned answer; it stays green when the real code breaks.",
        external_lints: &[],
        references: &[],
        fixes: &[LanguageBlock { language: "python", body: "Patch the collaborators the subject calls, or patch with `wraps=` to keep the real function." }, LanguageBlock { language: "typescript", body: "Mock the collaborators the subject imports, or keep the real module with `importActual` / `importOriginal`." }],
    },
];

pub fn all() -> impl Iterator<Item = &'static FindingDocs> {
    FINDINGS.iter()
}
