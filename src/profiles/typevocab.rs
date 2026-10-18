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
    /// A top type that forces narrowing before use (TS `unknown`).
    SafeTop,
}

/// Split `text` on `sep` outside any bracket pair, trimming each part.
pub(crate) fn split_top_level(text: &str, sep: char) -> Vec<&str> {
    let mut depth = BracketDepth::default();
    let mut parts = Vec::new();
    let mut start = 0;
    for (i, c) in text.char_indices() {
        match c {
            '<' | '[' | '(' | '{' => depth.open(),
            '>' | ']' | ')' | '}' => depth.close(),
            _ if c == sep && depth.is_top_level() => {
                parts.push(text[start..i].trim());
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

#[derive(Default)]
struct BracketDepth {
    value: usize,
}

impl BracketDepth {
    fn open(&mut self) {
        self.value += 1;
    }

    fn close(&mut self) {
        self.value = self.value.saturating_sub(1);
    }

    fn is_top_level(&self) -> bool {
        self.value == 0
    }
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
