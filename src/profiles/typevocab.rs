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

// ---- language routers -------------------------------------------------------

/// True when the annotation is a loose collection / escape hatch with no domain
/// type anywhere inside it. Routed to the language's own vocabulary.
pub fn is_loose(lang: Language, annotation: &str) -> bool {
    match lang {
        #[cfg(feature = "lang-python")]
        Language::Python => crate::profiles::python::typevocab::is_loose(annotation),
        #[cfg(feature = "lang-javascript")]
        Language::JavaScript | Language::TypeScript => {
            crate::profiles::javascript::typevocab::is_loose(annotation)
        }
        _ => false,
    }
}

/// The language's boolean type name (Python `bool`, TS `boolean`).
pub fn is_bool_type(lang: Language, annotation: &str) -> bool {
    match lang {
        #[cfg(feature = "lang-python")]
        Language::Python => crate::profiles::python::typevocab::is_bool(annotation),
        #[cfg(feature = "lang-javascript")]
        Language::JavaScript | Language::TypeScript => {
            crate::profiles::javascript::typevocab::is_bool(annotation)
        }
        _ => false,
    }
}

/// A dict/record-shaped (or `Any`-ish) annotation — a receiver that might carry
/// an implicit schema. Non-dict annotated receivers (DataFrame, ndarray) skip.
pub fn is_dictish(lang: Language, annotation: &str) -> bool {
    match lang {
        #[cfg(feature = "lang-python")]
        Language::Python => crate::profiles::python::typevocab::is_dictish(annotation),
        #[cfg(feature = "lang-javascript")]
        Language::JavaScript | Language::TypeScript => {
            crate::profiles::javascript::typevocab::is_dictish(annotation)
        }
        _ => false,
    }
}
