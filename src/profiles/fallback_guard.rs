//! Text-level check that a conditional guards the value it yields.

/// Per-language presence-test shapes.
pub(crate) struct GuardSyntax {
    /// `!!x`.
    pub(crate) truthy_prefixes: &'static [&'static str],
    /// `x != null`, `x is not None`.
    pub(crate) presence_suffixes: &'static [&'static str],
    /// `.`, `?.`, `[` after the tested value.
    pub(crate) accessors: &'static [&'static str],
}

/// `x ? x : ""`, `x != null ? x.name : ""`, `x.name if x is not None else ""`.
pub(crate) fn guards_own_value(condition: &str, value: &str, syntax: &GuardSyntax) -> bool {
    let subject = presence_subject(condition, syntax);
    !subject.is_empty()
        && value.strip_prefix(subject).is_some_and(|rest| {
            rest.is_empty() || syntax.accessors.iter().any(|a| rest.starts_with(a))
        })
}

fn presence_subject<'a>(condition: &'a str, syntax: &GuardSyntax) -> &'a str {
    let text = condition.trim();
    let stripped = strip_guard_once(text, syntax);
    if stripped == text {
        return text;
    }
    presence_subject(stripped, syntax)
}

/// One pass of paren, truthy-prefix and presence-suffix stripping.
fn strip_guard_once<'a>(text: &'a str, syntax: &GuardSyntax) -> &'a str {
    let unwrapped = text
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .unwrap_or(text)
        .trim();
    let unprefixed = syntax
        .truthy_prefixes
        .iter()
        .find_map(|prefix| unwrapped.strip_prefix(prefix))
        .unwrap_or(unwrapped)
        .trim();
    syntax
        .presence_suffixes
        .iter()
        .find_map(|suffix| unprefixed.strip_suffix(suffix))
        .unwrap_or(unprefixed)
        .trim()
}
