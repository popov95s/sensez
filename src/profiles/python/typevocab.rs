//! Python type-annotation vocabulary for the type-discipline smells. Shared
//! lexical helpers live in [`crate::profiles::typevocab`]; this module owns only
//! Python's conventions (`dict[str, Any]`, `Optional[...]`, `Literal`, ...).

use crate::profiles::typevocab::{
    base_type, has_domain_type, has_token, is_nominal_ignoring, split_top_level, LooseTypeKind,
};
use crate::spine::ir::MutatorFamily;
use tree_sitter::Node;

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
    if let Some(kind) = mapping_kind(annotation) {
        return Some(kind);
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

pub(crate) fn is_void(annotation: &str) -> bool {
    annotation.trim() == "None"
}

/// `Optional[T]` or `T | None`.
pub(crate) fn admits_missing(annotation: &str) -> bool {
    has_token(annotation, "Optional") || has_token(annotation, "None")
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

pub(crate) fn has_domain_model(annotation: &str) -> bool {
    has_domain_type(annotation, &TYPING_NAMES)
}

/// `Order`, `Optional[Order]`, `Order | None`.
pub(crate) fn is_nominal_type(annotation: &str) -> bool {
    let text = annotation.trim();
    let unwrapped = text
        .strip_prefix("typing.")
        .unwrap_or(text)
        .strip_prefix("Optional[")
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(text);
    is_nominal_ignoring(unwrapped, &["None"], &[])
}

pub(crate) fn is_primitive_scalar_alias(annotation: &str) -> bool {
    matches!(
        base_type(annotation),
        "str" | "int" | "float" | "bool" | "bytes"
    )
}

pub(crate) fn is_literal_union(annotation: &str) -> bool {
    base_type(annotation) == "Literal"
}

pub(crate) fn is_mutable_collection(annotation: &str, family: MutatorFamily) -> bool {
    match family {
        MutatorFamily::Array => matches!(base_type(annotation), "list" | "List" | "deque"),
        MutatorFamily::Collection => {
            matches!(base_type(annotation), "set" | "Set") || is_dictish(annotation)
        }
    }
}

/// `-> tuple[int, int]` / `-> Tuple[str, str, str]`: one primitive repeated.
pub(crate) fn is_homogeneous_primitive_tuple(func: Node, src: &[u8]) -> bool {
    let Some(annotation) = func
        .child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(src).ok())
    else {
        return false;
    };
    let text = annotation.trim();
    let text = text.strip_prefix("typing.").unwrap_or(text);
    let Some(body) = ["tuple[", "Tuple["]
        .iter()
        .find_map(|open| text.strip_prefix(open))
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return false;
    };
    let elements = split_top_level(body, ',');
    elements.first().is_some_and(|first| {
        matches!(*first, "int" | "str" | "float" | "bool" | "bytes")
            && elements.iter().all(|element| element == first)
    })
}

/// `Optional[dict[str, Any]]` → `dict[str, Any]` (best effort).
fn inner(annotation: &str) -> &str {
    annotation
        .split_once('[')
        .map(|(_, rest)| rest.strip_suffix(']').unwrap_or(rest))
        .unwrap_or("")
}

/// Bare or `object`-valued mappings erase the schema; typed values are primitive collections.
fn mapping_kind(annotation: &str) -> Option<LooseTypeKind> {
    let base = base_type(annotation);
    if base == "Optional" {
        return mapping_kind(inner(annotation));
    }
    if !matches!(base, "dict" | "Dict" | "Mapping" | "MutableMapping") {
        return None;
    }
    let value = split_top_level(inner(annotation), ',').get(1).copied();
    match value {
        None | Some("object") => Some(LooseTypeKind::SchemaErasing),
        Some(_) => Some(LooseTypeKind::PrimitiveCollection),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualified_optional_is_unwrapped_before_the_nominal_check() {
        assert!(is_nominal_type("typing.Optional[Order]"));
        assert!(!is_nominal_type("typing.Optional[Order | Invoice]"));
    }

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
