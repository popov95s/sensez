//! Rust type-annotation vocabulary for smell detectors.

use crate::profiles::typevocab::{base_type, has_domain_type, idents, LooseTypeKind};
use crate::spine::ir::MutatorFamily;

const BUILTINS: &[&str] = &[
    "Vec", "HashMap", "BTreeMap", "HashSet", "BTreeSet", "Option", "Result", "Box", "Rc", "Arc",
    "String", "str", "bool", "usize", "isize", "u8", "u16", "u32", "u64", "u128", "i8", "i16",
    "i32", "i64", "i128", "f32", "f64",
];

pub fn loose_kind(annotation: &str) -> Option<LooseTypeKind> {
    if idents(annotation).any(|t| t == "dyn") {
        return Some(LooseTypeKind::EscapeHatch);
    }
    let base = base_type(annotation).trim_start_matches('&').trim();
    if matches!(base, "HashMap" | "BTreeMap") && !has_domain_type(annotation, BUILTINS) {
        return Some(LooseTypeKind::SchemaErasing);
    }
    if base == "Vec" && !has_domain_type(annotation, BUILTINS) {
        return Some(LooseTypeKind::PrimitiveCollection);
    }
    None
}

pub fn is_void(annotation: &str) -> bool {
    annotation.trim() == "()"
}

pub fn is_bool(annotation: &str) -> bool {
    annotation.trim_start_matches('&').trim() == "bool"
}

pub fn is_dictish(annotation: &str) -> bool {
    let base = base_type(annotation).trim_start_matches('&').trim();
    matches!(base, "HashMap" | "BTreeMap")
}

pub fn is_mutable_collection(annotation: &str, family: MutatorFamily) -> bool {
    let base = base_type(annotation)
        .trim_start_matches('&')
        .trim_start_matches("mut ")
        .trim();
    match family {
        MutatorFamily::Array => matches!(base, "Vec" | "VecDeque"),
        MutatorFamily::Collection => {
            matches!(base, "HashSet" | "BTreeSet") || is_dictish(annotation)
        }
    }
}

pub fn has_domain_model(annotation: &str) -> bool {
    has_domain_type(annotation, BUILTINS)
}

pub fn is_primitive_scalar_alias(annotation: &str) -> bool {
    matches!(
        base_type(annotation).trim_start_matches('&').trim(),
        "String"
            | "str"
            | "bool"
            | "usize"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "f32"
            | "f64"
    )
}
