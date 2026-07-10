//! TypeScript/JavaScript type-annotation vocabulary for the type-discipline
//! smells. Shared lexical helpers live in [`crate::profiles::typevocab`]; this
//! module owns only the TS/JS conventions (`Record<string, any>`, `any`,
//! `Set<number>`, ...). TypeScript reuses this module (it reuses the JS walker).

use crate::profiles::typevocab::{base_type, has_domain_type, has_token, idents, LooseTypeKind};

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
    if has_domain_type(annotation, &BUILTINS) {
        return None;
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

fn primitive_array(annotation: &str) -> bool {
    let trimmed = annotation.trim();
    trimmed.ends_with("[]") || matches!(base_type(trimmed), "Array" | "ReadonlyArray")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
