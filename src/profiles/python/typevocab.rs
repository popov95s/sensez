//! Python type-annotation vocabulary for the type-discipline smells. Shared
//! lexical helpers live in [`crate::profiles::typevocab`]; this module owns only
//! Python's conventions (`dict[str, Any]`, `Optional[...]`, `Literal`, ...).

use crate::profiles::typevocab::{base_type, has_domain_type, has_token, LooseTypeKind};

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

pub(crate) fn loose_kind(annotation: &str) -> Option<LooseTypeKind> {
    if has_token(annotation, "Any") {
        return Some(LooseTypeKind::EscapeHatch);
    }
    if annotation.contains("Literal") || has_domain_type(annotation, &TYPING_NAMES) {
        return None;
    }
    if is_schema_erasing(annotation) {
        return Some(LooseTypeKind::SchemaErasing);
    }
    let primitive_collection = annotation
        .split('|')
        .any(|part| LOOSE_BASES.contains(&base_type(part)) && base_type(part) != "Optional")
        || base_type(annotation) == "Optional"
            && loose_kind(inner(annotation))
                .is_some_and(|kind| matches!(kind, LooseTypeKind::PrimitiveCollection));
    primitive_collection.then_some(LooseTypeKind::PrimitiveCollection)
}

/// `list[str]` / `dict[str, Any]` / `set[int]` are loose; `list[UserModel]` /
/// `Literal["json", "xml"]` are typed and never match.
#[cfg(test)]
pub(crate) fn is_loose(annotation: &str) -> bool {
    loose_kind(annotation).is_some()
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

fn is_schema_erasing(annotation: &str) -> bool {
    let base = base_type(annotation);
    matches!(base, "dict" | "Dict" | "Mapping" | "MutableMapping")
        || base == "Optional" && is_schema_erasing(inner(annotation))
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
