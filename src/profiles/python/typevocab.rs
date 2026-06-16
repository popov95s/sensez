//! Python type-annotation vocabulary for the type-discipline smells. Shared
//! lexical helpers live in [`crate::profiles::typevocab`]; this module owns only
//! Python's conventions (`dict[str, Any]`, `Optional[...]`, `Literal`, ...).

use crate::profiles::typevocab::{base_type, has_domain_type};

/// Annotation bases that are loose collections (a missing domain type).
/// Sets are included: `set[int]` / `set[str]` is the same "collection of bare
/// primitives" smell as `list[int]` — `set[UserModel]` is spared by the
/// domain-type check, exactly like the other containers.
const LOOSE_BASES: [&str; 12] = [
    "dict",
    "Dict",
    "list",
    "List",
    "tuple",
    "Tuple",
    "set",
    "Set",
    "frozenset",
    "FrozenSet",
    "Any",
    "Optional",
];

/// Typing-module names that are containers/wrappers, not domain types.
/// `Literal` is deliberately absent: a `Literal` annotation is disciplined.
const TYPING_NAMES: [&str; 16] = [
    "Dict",
    "List",
    "Tuple",
    "Set",
    "FrozenSet",
    "Optional",
    "Union",
    "Any",
    "Sequence",
    "Mapping",
    "MutableMapping",
    "Iterable",
    "Iterator",
    "Callable",
    "Awaitable",
    "None",
];

/// `list[str]` / `dict[str, Any]` / `set[int]` are loose; `list[UserModel]` /
/// `Literal["json", "xml"]` are typed and never match.
pub(crate) fn is_loose(annotation: &str) -> bool {
    if annotation.contains("Literal") || has_domain_type(annotation, &TYPING_NAMES) {
        return false;
    }
    annotation
        .split('|')
        .any(|part| LOOSE_BASES.contains(&base_type(part)) && base_type(part) != "Optional")
        || base_type(annotation) == "Optional" && is_loose(inner(annotation))
}

pub(crate) fn is_bool(annotation: &str) -> bool {
    base_type(annotation) == "bool"
}

pub(crate) fn is_dictish(annotation: &str) -> bool {
    matches!(
        base_type(annotation),
        "dict" | "Dict" | "Mapping" | "MutableMapping" | "Any"
    )
}

/// `Optional[dict[str, Any]]` → `dict[str, Any]` (best effort).
fn inner(annotation: &str) -> &str {
    annotation
        .split_once('[')
        .map(|(_, rest)| rest.strip_suffix(']').unwrap_or(rest))
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_of_primitives_are_loose_but_sets_of_domain_types_are_not() {
        assert!(is_loose("set[int]"));
        assert!(is_loose("set[str]"));
        assert!(is_loose("Set[int]"));
        assert!(is_loose("frozenset[str]"));
        assert!(is_loose("Optional[set[int]]"));
        assert!(!is_loose("set[UserModel]"));
        // Parity with the existing containers stays intact.
        assert!(is_loose("list[int]"));
        assert!(!is_loose("list[UserModel]"));
        assert!(!is_loose("Literal[\"json\", \"xml\"]"));
    }

    #[test]
    fn bool_and_dictish() {
        assert!(is_bool("bool"));
        assert!(!is_bool("boolean"));
        assert!(is_dictish("dict[str, int]"));
        assert!(!is_dictish("DataFrame"));
    }
}
