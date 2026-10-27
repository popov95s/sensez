//! Literal lists taken from an outside vocabulary (file extensions, MIME types,
//! paths, error or status codes) rather than the codebase's own categories.

pub(super) fn is_external_vocabulary(literals: &[String]) -> bool {
    !literals.is_empty() && literals.iter().all(|literal| is_external(literal))
}

/// The tested value is a tag name read through `markup_names`: `tag`, `el.tagName`, `local(node)`.
pub(super) fn is_markup_operand(operand: &str, markup_names: &[&str]) -> bool {
    let callee = operand.split('(').next().unwrap_or(operand);
    let last = callee.rsplit('.').next().unwrap_or(callee);
    markup_names.contains(&last)
}

fn is_external(literal: &str) -> bool {
    is_punctuation(literal)
        || is_extension(literal)
        || literal.contains('/')
        || is_constant_code(literal)
        || literal.starts_with(|c: char| c.is_ascii_digit())
}

/// `" "`, `"."`, `"["`.
fn is_punctuation(literal: &str) -> bool {
    let mut chars = literal.chars();
    matches!((chars.next(), chars.next()), (Some(c), None) if !c.is_alphanumeric())
}

/// `.png`, `.tar`.
fn is_extension(literal: &str) -> bool {
    literal
        .strip_prefix('.')
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// `ENOENT`, `E_ACCESS`.
fn is_constant_code(literal: &str) -> bool {
    literal.len() > 1
        && literal.starts_with(|c: char| c.is_ascii_uppercase())
        && literal
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}
