//! Type-annotation vocabulary: the language-neutral seam + shared lexical base.
//!
//! Each language's notion of a "loose" / boolean / dict-shaped annotation lives
//! in its own profile module (`python::typevocab`, `javascript::typevocab`) —
//! never jammed together. This module owns only (a) the small string helpers
//! every language reuses, and (b) thin routers that dispatch a [`Language`] to
//! its module (the same compile-time, feature-gated routing as [`registry`]).
//! Analyzer passes call these routers and stay language-agnostic.
//!
//! [`registry`]: crate::profiles::registry

use crate::spine::ir::Language;

// ---- shared lexical helpers (used by every language's vocab) ----------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LooseTypeKind {
    EscapeHatch,
    SchemaErasing,
    PrimitiveCollection,
}

/// Base name of an annotation: `dict[str, Any]` → `dict`,
/// `Record<string, any>` → `Record`, `any[]` → `any`. Splits on the first
/// generic/subscript bracket (`[` or `<`) so both syntaxes work.
pub fn base_type(annotation: &str) -> &str {
    annotation
        .split(['[', '<'])
        .next()
        .unwrap_or(annotation)
        .trim()
}

/// Identifier-ish tokens of an annotation (`_`-joined alphanumerics).
pub(crate) fn idents(annotation: &str) -> impl Iterator<Item = &str> {
    annotation
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
}

/// Any capitalized identifier that isn't a known container/builtin name — i.e. a
/// dataclass/model/interface appears somewhere in the annotation.
pub(crate) fn has_domain_type(annotation: &str, builtins: &[&str]) -> bool {
    idents(annotation)
        .any(|t| t.chars().next().is_some_and(char::is_uppercase) && !builtins.contains(&t))
}

pub(crate) fn has_token(annotation: &str, token: &str) -> bool {
    idents(annotation).any(|t| t == token)
}

pub fn is_primitive_scalar_alias(lang: Language, target: &str) -> bool {
    let base = base_type(target).trim_start_matches('&').trim();
    match lang {
        Language::Python => matches!(base, "str" | "int" | "float" | "bool" | "bytes"),
        Language::JavaScript | Language::TypeScript => {
            matches!(base, "string" | "number" | "boolean" | "bigint" | "symbol")
        }
        Language::Rust => matches!(
            base,
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
        ),
    }
}

// ---- language routers -------------------------------------------------------

#[allow(unreachable_patterns)]
pub fn loose_kind(lang: Language, annotation: &str) -> Option<LooseTypeKind> {
    match lang {
        #[cfg(feature = "lang-python")]
        Language::Python => crate::profiles::python::typevocab::loose_kind(annotation),
        #[cfg(feature = "lang-javascript")]
        Language::JavaScript | Language::TypeScript => {
            crate::profiles::javascript::typevocab::loose_kind(annotation)
        }
        #[cfg(feature = "lang-rust")]
        Language::Rust => crate::profiles::rust::typevocab::loose_kind(annotation),
        _ => None,
    }
}

/// The language's boolean type name (Python `bool`, TS `boolean`).
#[allow(unreachable_patterns)]
pub fn is_bool_type(lang: Language, annotation: &str) -> bool {
    match lang {
        #[cfg(feature = "lang-python")]
        Language::Python => crate::profiles::python::typevocab::is_bool(annotation),
        #[cfg(feature = "lang-javascript")]
        Language::JavaScript | Language::TypeScript => {
            crate::profiles::javascript::typevocab::is_bool(annotation)
        }
        #[cfg(feature = "lang-rust")]
        Language::Rust => crate::profiles::rust::typevocab::is_bool(annotation),
        _ => false,
    }
}

/// A dict/record-shaped (or `Any`-ish) annotation — a receiver that might carry
/// an implicit schema. Non-dict annotated receivers (DataFrame, ndarray) skip.
#[allow(unreachable_patterns)]
pub fn is_dictish(lang: Language, annotation: &str) -> bool {
    match lang {
        #[cfg(feature = "lang-python")]
        Language::Python => crate::profiles::python::typevocab::is_dictish(annotation),
        #[cfg(feature = "lang-javascript")]
        Language::JavaScript | Language::TypeScript => {
            crate::profiles::javascript::typevocab::is_dictish(annotation)
        }
        #[cfg(feature = "lang-rust")]
        Language::Rust => crate::profiles::rust::typevocab::is_dictish(annotation),
        _ => false,
    }
}
