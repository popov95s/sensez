//! Language-neutral mechanics shared by type-vocabulary profiles.
//!
//! Language-specific annotation names and classifications belong exclusively
//! to each language's `typevocab` module and are exposed through
//! [`super::TypeVocabularyProfile`].

// ---- shared lexical helpers (used by every language's vocab) ----------------

/// `ch` continues an identifier: alphanumeric, `_`, or one of the language's `extra` characters.
pub(crate) fn is_ident_char(ch: char, extra: &[char]) -> bool {
    ch.is_alphanumeric() || ch == '_' || extra.contains(&ch)
}

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
    let mut depth = 0usize;
    let mut parts = Vec::new();
    let mut start = 0;
    let mut previous = None;
    for (i, c) in text.char_indices() {
        let arrow = previous == Some('=');
        previous = Some(c);
        match c {
            '>' if arrow => {}
            '<' | '[' | '(' | '{' => depth += 1,
            '>' | ']' | ')' | '}' => depth = depth.saturating_sub(1),
            _ if c == sep && depth == 0 => {
                parts.push(text[start..i].trim());
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
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

/// [`is_nominal`] after dropping the language's nullish union members (`Order | None`).
pub(crate) fn is_nominal_ignoring(
    annotation: &str,
    nullish: &[&str],
    extra_chars: &[char],
) -> bool {
    let mut members = split_top_level(annotation, '|')
        .into_iter()
        .filter(|member| !nullish.contains(member));
    matches!((members.next(), members.next()), (Some(only), None) if is_nominal(only, extra_chars))
}

/// `Order`, `pkg.Order`, `Repo<User>`, `list[Order]`; not inline literals, unions or intersections.
pub(crate) fn is_nominal(annotation: &str, extra_chars: &[char]) -> bool {
    let text = annotation.trim();
    let combined = |sep| split_top_level(text, sep).len() > 1;
    if text.contains(['{', '\n']) || combined('|') || combined('&') {
        return false;
    }
    let head = base_type(text);
    let generic = &text[head.len()..];
    head.starts_with(|c: char| !c.is_numeric() && is_ident_char(c, extra_chars))
        && head
            .chars()
            .all(|c| c == '.' || is_ident_char(c, extra_chars))
        && (generic.is_empty() || generic.ends_with(['>', ']']))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_inside_generic_does_not_close_a_bracket() {
        assert_eq!(
            split_top_level("Foo<A<() => B, C>>, D", ','),
            vec!["Foo<A<() => B, C>>", "D"]
        );
    }

    #[test]
    fn union_inside_generic_stays_nominal() {
        assert!(is_nominal_ignoring("Repo<A | B> | null", &["null"], &[]));
        assert!(!is_nominal("A | B", &[]));
    }
}
