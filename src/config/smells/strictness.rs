//! Shared metadata for rule knobs with ordered strictness levels.

use super::knobs::Strictness;
use crate::report::SmellKind;

#[allow(dead_code)] // Read by docs/rust_metadata.py as source-level metadata.
pub struct StrictnessLevelDoc {
    pub title: &'static str,
    pub value: &'static str,
    pub description: &'static str,
}

pub struct StrictnessRuleDoc {
    pub kind: SmellKind,
    pub knob: &'static str,
    pub levels: &'static [StrictnessLevelDoc],
}

pub const LOOSE_TYPING_LEVELS: &[StrictnessLevelDoc] = &[
    StrictnessLevelDoc {
        title: "Low",
        value: "low",
        description: "Flags only direct escape hatches.",
    },
    StrictnessLevelDoc {
        title: "Medium",
        value: "medium",
        description: "Flags escape hatches plus schema-erasing maps and records.",
    },
    StrictnessLevelDoc {
        title: "High",
        value: "high",
        description: "Flags everything in medium, primitive-only collections, bare `unknown` params, and shallow aliases.",
    },
];

pub const MAGIC_STRING_DEFAULT_LEVELS: &[StrictnessLevelDoc] = &[
    StrictnessLevelDoc {
        title: "Low",
        value: "low",
        description: "Flags empty, one-character, or sentinel-word fallbacks that are stored, sent, or compared.",
    },
    StrictnessLevelDoc {
        title: "Medium",
        value: "medium",
        description: "Also flags empty or one-character fallbacks that are returned or kept in a variable; skips rendered text and string operations, including coercions such as `Number(..)` and lookups such as `map.get(..)`.",
    },
    StrictnessLevelDoc {
        title: "High",
        value: "high",
        description: "Flags every empty, one-character, or sentinel-word fallback wherever it flows.",
    },
];

pub const NESTED_TERNARY_LEVELS: &[StrictnessLevelDoc] = &[
    StrictnessLevelDoc {
        title: "Medium",
        value: "medium",
        description: "Flags direct chains, where one conditional is a branch or condition of another.",
    },
    StrictnessLevelDoc {
        title: "High",
        value: "high",
        description: "Also flags conditionals nested inside a call argument, template, JSX, or object literal of another.",
    },
];

pub const SWALLOWED_ERROR_LEVELS: &[StrictnessLevelDoc] = &[
    StrictnessLevelDoc {
        title: "Low",
        value: "low",
        description: "Flags handlers that return an empty value (`null`, `None`, `[]`, `false`), except around a single JSON parse.",
    },
    StrictnessLevelDoc {
        title: "Medium",
        value: "medium",
        description: "Also flags empty-value returns around a single JSON parse.",
    },
    StrictnessLevelDoc {
        title: "High",
        value: "high",
        description: "Also flags handlers that only log or ignore the error, and handlers that return another literal default such as `\"unknown\"`.",
    },
];

pub const MOCKED_SUBJECT_LEVELS: &[StrictnessLevelDoc] = &[
    StrictnessLevelDoc {
        title: "Medium",
        value: "medium",
        description: "Flags a mock whose module resolves to the import a test calls its subject through.",
    },
    StrictnessLevelDoc {
        title: "High",
        value: "high",
        description: "Also flags a mocked module named like the test file, and patches of an attribute on an imported object the test calls.",
    },
];

pub const TYPE_ESCAPE_CAST_LEVELS: &[StrictnessLevelDoc] = &[
    StrictnessLevelDoc {
        title: "Low",
        value: "low",
        description: "Flags casts to the escape type (`as any`, `cast(Any, …)`) and bare `@ts-ignore`.",
    },
    StrictnessLevelDoc {
        title: "Medium",
        value: "medium",
        description: "Also flags `as unknown as T`, bare `@ts-expect-error`, and `# type: ignore` without an error code.",
    },
    StrictnessLevelDoc {
        title: "High",
        value: "high",
        description: "Also flags non-null assertions on call results (`find()!`).",
    },
];

pub const NAME_PROMISE_LEVELS: &[StrictnessLevelDoc] = &[
    StrictnessLevelDoc {
        title: "Low",
        value: "low",
        description: "Flags only database, network and scheduling writes.",
    },
    StrictnessLevelDoc {
        title: "Medium",
        value: "medium",
        description: "Also flags mutating calls and assignments on parameters, own members and module-level names; skips caches and lazy initialisation.",
    },
    StrictnessLevelDoc {
        title: "High",
        value: "high",
        description: "Also flags cache fills and lazy initialisation, and treats `load` and `fetch` names as reads.",
    },
];

pub const STRICTNESS_RULES: &[StrictnessRuleDoc] = &[
    StrictnessRuleDoc {
        kind: SmellKind::LooseTyping,
        knob: "strictness",
        levels: LOOSE_TYPING_LEVELS,
    },
    StrictnessRuleDoc {
        kind: SmellKind::MagicStringDefault,
        knob: "strictness",
        levels: MAGIC_STRING_DEFAULT_LEVELS,
    },
    StrictnessRuleDoc {
        kind: SmellKind::NestedTernary,
        knob: "strictness",
        levels: NESTED_TERNARY_LEVELS,
    },
    StrictnessRuleDoc {
        kind: SmellKind::SwallowedErrorSentinel,
        knob: "strictness",
        levels: SWALLOWED_ERROR_LEVELS,
    },
    StrictnessRuleDoc {
        kind: SmellKind::MockedSubject,
        knob: "strictness",
        levels: MOCKED_SUBJECT_LEVELS,
    },
    StrictnessRuleDoc {
        kind: SmellKind::TypeEscapeCast,
        knob: "strictness",
        levels: TYPE_ESCAPE_CAST_LEVELS,
    },
    StrictnessRuleDoc {
        kind: SmellKind::NamePromiseMismatch,
        knob: "strictness",
        levels: NAME_PROMISE_LEVELS,
    },
];

pub fn is_string_rule_knob(kind: SmellKind, key: &str) -> bool {
    strictness_rule(kind, key).is_some()
}

pub fn valid_values(kind: SmellKind, key: &str) -> Option<String> {
    let values = strictness_rule(kind, key)?
        .levels
        .iter()
        .map(|level| level.value)
        .collect::<Vec<_>>()
        .join(", ");
    Some(values)
}

pub fn parse_strictness(kind: SmellKind, key: &str, value: &str) -> Result<Strictness, String> {
    let Some(rule) = strictness_rule(kind, key) else {
        return Err(format!(
            "[smells.rules.{kind}] {key} is not a strictness knob"
        ));
    };
    let listed = rule.levels.iter().any(|level| level.value == value);
    match value {
        "low" if listed => Ok(Strictness::Low),
        "medium" if listed => Ok(Strictness::Medium),
        "high" if listed => Ok(Strictness::High),
        _ => Err(format!(
            "[smells.rules.{kind}] {key} must be one of: {}",
            valid_values(kind, key).unwrap_or_default()
        )),
    }
}

fn strictness_rule(kind: SmellKind, key: &str) -> Option<&'static StrictnessRuleDoc> {
    STRICTNESS_RULES
        .iter()
        .find(|rule| rule.kind == kind && rule.knob == key)
}
