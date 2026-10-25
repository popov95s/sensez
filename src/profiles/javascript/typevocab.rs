//! TypeScript/JavaScript type-annotation vocabulary for the type-discipline
//! smells. Shared lexical helpers live in [`crate::profiles::typevocab`]; this
//! module owns only the TS/JS conventions (`Record<string, any>`, `any`,
//! `Set<number>`, ...). TypeScript reuses this module (it reuses the JS walker).

use crate::profiles::typevocab::{
    base_type, has_domain_type, has_token, idents, is_nominal_ignoring, split_top_level,
    LooseTypeKind,
};
use crate::spine::ir::MutatorFamily;

/// Loose escape-hatch identifiers: `any`/`unknown`/`object` and the built-in
/// untyped containers. Their presence (with no domain type alongside) is loose.
const LOOSE_TOKENS: [&str; 11] = [
    "any", "unknown", "object", "Object", "Function", "Record", "Map", "Dict", "WeakMap", "Set",
    "WeakSet",
];

const SCHEMA_ERASING: [&str; 7] = [
    "unknown", "object", "Object", "Function", "Record", "Map", "Dict",
];

/// Built-in / utility type names that are NOT domain types.
const BUILTINS: [&str; 22] = [
    "string",
    "number",
    "boolean",
    "bigint",
    "symbol",
    "void",
    "never",
    "undefined",
    "null",
    "unknown",
    "any",
    "object",
    "Object",
    "Function",
    "Array",
    "ReadonlyArray",
    "Record",
    "Map",
    "Set",
    "WeakMap",
    "Promise",
    "Date",
];

pub(crate) fn loose_kind(annotation: &str) -> Option<LooseTypeKind> {
    if has_token(annotation, "any") {
        return Some(LooseTypeKind::EscapeHatch);
    }
    if matches!(
        non_nullish(annotation).as_slice(),
        ["unknown" | "unknown[]"]
    ) {
        return Some(LooseTypeKind::SafeTop);
    }
    if has_domain_type(annotation, &BUILTINS) {
        return None;
    }
    if let Some(value) = map_value_type(annotation) {
        return Some(map_kind(value));
    }
    if idents(annotation).any(|t| SCHEMA_ERASING.contains(&t)) {
        return Some(LooseTypeKind::SchemaErasing);
    }
    if primitive_array(annotation) || idents(annotation).any(|t| LOOSE_TOKENS.contains(&t)) {
        return Some(LooseTypeKind::PrimitiveCollection);
    }
    None
}

/// `Record<string, any>` / `any` / `Set<number>` are loose;
/// `Record<string, UserDto>` / `User[]` / `string` are typed and never match.
#[cfg(test)]
pub(crate) fn is_loose(annotation: &str) -> bool {
    loose_kind(annotation).is_some()
}

pub(crate) fn is_bool(annotation: &str) -> bool {
    base_type(annotation) == "boolean"
}

pub(crate) fn is_dictish(annotation: &str) -> bool {
    matches!(
        base_type(annotation),
        "Record" | "Map" | "object" | "any" | "unknown"
    ) || annotation.trim_start().starts_with('{')
}

/// `Record<"a" | "b", V>`, `Record<Key, V>`, `Record<string, number>`, `{ [k: string]: string }`.
pub(crate) fn is_open_map(annotation: &str) -> bool {
    let text = annotation.trim();
    if let Some(args) = text
        .strip_prefix("Record<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        return match split_top_level(args, ',')[..] {
            [key, value] => {
                !matches!(key, "string" | "number" | "symbol" | "PropertyKey")
                    || is_primitive_scalar_alias(value)
            }
            _ => false,
        };
    }
    text.strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .and_then(|body| body.trim().trim_end_matches([';', ',']).strip_prefix('['))
        .and_then(|signature| signature.split_once(']'))
        .and_then(|(_, rest)| rest.trim_start().strip_prefix(':'))
        .is_some_and(is_primitive_scalar_alias)
}

/// `Order`, `Order | undefined`, `Order | null`.
pub(crate) fn is_nominal_type(annotation: &str) -> bool {
    is_nominal_ignoring(
        annotation,
        &["null", "undefined"],
        super::wording::WORDING.identifier_extra_chars,
    )
}

pub(crate) fn has_domain_model(annotation: &str) -> bool {
    has_domain_type(annotation, &BUILTINS)
}

pub(crate) fn is_primitive_scalar_alias(annotation: &str) -> bool {
    matches!(
        base_type(annotation),
        "string" | "number" | "boolean" | "bigint" | "symbol"
    )
}

/// `value is User` / `asserts value is User` / `asserts value` → `value`.
pub(crate) fn guarded_param(return_annotation: &str) -> Option<&str> {
    let mut words = return_annotation.split_whitespace();
    match (words.next()?, words.next()) {
        ("asserts", Some(subject)) => Some(subject),
        (subject, Some("is")) => Some(subject),
        _ => None,
    }
}

/// `"a" | "b"`, `keyof typeof X` or `(typeof X)[number]`.
pub(crate) fn is_literal_union(annotation: &str) -> bool {
    let trimmed = annotation.trim();
    if trimmed.starts_with("keyof typeof ")
        || (trimmed.starts_with("(typeof ") && trimmed.ends_with(")[number]"))
    {
        return true;
    }
    let parts = non_nullish(trimmed);
    !parts.is_empty() && parts.iter().all(|part| is_string_literal(part))
}

pub(crate) fn is_mutable_collection(annotation: &str, family: MutatorFamily) -> bool {
    non_nullish(annotation).iter().any(|part| match family {
        MutatorFamily::Array => part.ends_with("[]") || base_type(part) == "Array",
        MutatorFamily::Collection => {
            matches!(base_type(part), "Map" | "Set" | "WeakMap" | "WeakSet") || is_dictish(part)
        }
    })
}

/// `void`, `undefined`, or an async `Promise<void>`.
pub(crate) fn is_void(annotation: &str) -> bool {
    let text = annotation.trim();
    let resolved = text
        .strip_prefix("Promise<")
        .and_then(|rest| rest.strip_suffix('>'))
        .unwrap_or(text);
    matches!(resolved.trim(), "void" | "undefined")
}

/// `T | null`, `T | undefined`, also inside `Promise<...>`.
pub(crate) fn admits_missing(annotation: &str) -> bool {
    has_token(annotation, "null") || has_token(annotation, "undefined")
}

/// Union members other than `undefined` / `null`.
fn non_nullish(annotation: &str) -> Vec<&str> {
    split_top_level(annotation, '|')
        .into_iter()
        .filter(|part| !matches!(*part, "undefined" | "null"))
        .collect()
}

fn is_string_literal(part: &str) -> bool {
    part.len() >= 2
        && ['"', '\'', '`']
            .iter()
            .any(|q| part.starts_with(*q) && part.ends_with(*q))
}

/// Value type argument of `Record<K, V>` / `Map<K, V>` / `WeakMap<K, V>`.
fn map_value_type(annotation: &str) -> Option<&str> {
    let trimmed = annotation.trim();
    if !matches!(base_type(trimmed), "Record" | "Map" | "WeakMap") {
        return None;
    }
    let args = trimmed.split_once('<')?.1.strip_suffix('>')?;
    split_top_level(args, ',').get(1).copied()
}

fn map_kind(value: &str) -> LooseTypeKind {
    match loose_kind(value) {
        Some(LooseTypeKind::SchemaErasing | LooseTypeKind::SafeTop) => LooseTypeKind::SchemaErasing,
        _ if value == "{}" => LooseTypeKind::SchemaErasing,
        _ => LooseTypeKind::PrimitiveCollection,
    }
}

fn primitive_array(annotation: &str) -> bool {
    let trimmed = annotation.trim();
    trimmed.ends_with("[]") || matches!(base_type(trimmed), "Array" | "ReadonlyArray")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promise_of_void_is_void() {
        assert!(is_void("Promise<void>"));
        assert!(!is_void("Promise<Order>"));
    }

    #[test]
    fn open_map_key_with_nested_generic_is_read_whole() {
        assert!(is_open_map("Record<Map<K, V>, Order>"));
        assert!(!is_open_map("Record<string, Map<K, V>>"));
    }

    #[test]
    fn sets_and_records_mirror_python_loose_collections() {
        assert!(is_loose("Set<number>"));
        assert!(is_loose("Record<string, any>"));
        assert!(!is_loose("Set<UserDto>"));
        assert!(!is_loose("User[]"));
        assert!(!is_loose("string"));
    }

    #[test]
    fn bool_and_dictish() {
        assert!(is_bool("boolean"));
        assert!(!is_bool("bool"));
        assert!(is_dictish("Record<string, number>"));
        assert!(!is_dictish("UserDto"));
    }

    #[test]
    fn open_maps_have_typed_keys_or_primitive_values() {
        assert!(is_open_map(r#"Record<"a" | "b" | "c", number>"#));
        assert!(is_open_map("Record<Category, Item>"));
        assert!(is_open_map("Record<string, number>"));
        assert!(is_open_map("{ [k: string]: string }"));
        assert!(!is_open_map("Record<string, any>"));
        assert!(!is_open_map("{ [k: string]: Item }"));
        assert!(!is_open_map("{ host: string; port: number }"));
        assert!(!is_open_map("any"));
    }
}
