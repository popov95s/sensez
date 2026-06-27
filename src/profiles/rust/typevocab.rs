//! Rust type-annotation vocabulary for smell detectors.

use crate::profiles::typevocab::{base_type, has_domain_type, idents};

const BUILTINS: &[&str] = &[
    "Vec", "HashMap", "BTreeMap", "HashSet", "BTreeSet", "Option", "Result", "Box", "Rc", "Arc",
    "String", "str", "bool", "usize", "isize", "u8", "u16", "u32", "u64", "u128", "i8", "i16",
    "i32", "i64", "i128", "f32", "f64",
];

pub fn is_loose(annotation: &str) -> bool {
    let base = base_type(annotation).trim_start_matches('&').trim();
    matches!(base, "Vec" | "HashMap" | "BTreeMap") && !has_domain_type(annotation, BUILTINS)
        || idents(annotation).any(|t| t == "dyn")
}

pub fn is_bool(annotation: &str) -> bool {
    annotation.trim_start_matches('&').trim() == "bool"
}

pub fn is_dictish(annotation: &str) -> bool {
    let base = base_type(annotation).trim_start_matches('&').trim();
    matches!(base, "HashMap" | "BTreeMap")
}
