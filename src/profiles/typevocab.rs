//! Language-neutral mechanics shared by type-vocabulary profiles.
//!
//! Language-specific annotation names and classifications belong exclusively
//! to each language's `typevocab` module and are exposed through
//! [`super::TypeVocabularyProfile`].

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
