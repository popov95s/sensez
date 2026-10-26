//! Text-level check that a conditional guards the value it yields.

use super::node_query::node_text;
use tree_sitter::Node;

/// Per-language presence-test shapes.
pub(crate) struct GuardSyntax {
    /// `!!x`.
    pub(crate) truthy_prefixes: &'static [&'static str],
    /// `x != null`, `x is not None`.
    pub(crate) presence_suffixes: &'static [&'static str],
    /// `!x`, `not x`.
    pub(crate) absence_prefixes: &'static [&'static str],
    /// `x == null`, `x is None`.
    pub(crate) absence_suffixes: &'static [&'static str],
    /// `.`, `?.`, `[` after the tested value.
    pub(crate) accessors: &'static [&'static str],
}

/// The fallback branch of `condition ? then : otherwise`, if either branch is the guarded value.
pub(crate) fn fallback_branch<'t>(
    condition: Node<'t>,
    then: Node<'t>,
    otherwise: Node<'t>,
    src: &[u8],
    syntax: &GuardSyntax,
) -> Option<Node<'t>> {
    let text = |node: Node| node_text(node, src).unwrap_or_default();
    let tested = text(condition);
    if yields(presence_subject(tested, syntax), text(then), syntax) {
        return Some(otherwise);
    }
    absence_subject(tested, syntax)
        .is_some_and(|subject| yields(subject, text(otherwise), syntax))
        .then_some(then)
}

/// `x` and `x.name` both yield the tested `x`.
fn yields(subject: &str, value: &str, syntax: &GuardSyntax) -> bool {
    !subject.is_empty()
        && value.strip_prefix(subject).is_some_and(|rest| {
            rest.is_empty() || syntax.accessors.iter().any(|a| rest.starts_with(a))
        })
}

/// `x` in `!x`, `x == null`, `not x`, `x is None`; never `!!x`.
fn absence_subject<'a>(condition: &'a str, syntax: &GuardSyntax) -> Option<&'a str> {
    let text = unparen(condition.trim());
    if syntax.truthy_prefixes.iter().any(|p| text.starts_with(p)) {
        return None;
    }
    let negated = syntax
        .absence_prefixes
        .iter()
        .find_map(|p| text.strip_prefix(p));
    let compared = || {
        syntax
            .absence_suffixes
            .iter()
            .find_map(|s| text.strip_suffix(s))
    };
    negated
        .or_else(compared)
        .map(|subject| unparen(subject.trim()))
}

fn unparen(text: &str) -> &str {
    text.strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .map_or(text, str::trim)
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
    let unwrapped = unparen(text);
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
