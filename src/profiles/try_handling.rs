//! Shared try/handler shapes behind defensive-fallback facts.

use tree_sitter::Node;

/// Per-language try/handler syntax and logger callees.
pub(crate) struct TrySyntax {
    pub(crate) try_kind: &'static str,
    pub(crate) handler_kind: &'static str,
    pub(crate) throw_kinds: &'static [&'static str],
    pub(crate) call_kind: &'static str,
    /// Callee texts; a leading or trailing `*` matches any prefix or suffix.
    pub(crate) loggers: &'static [&'static str],
    pub(crate) is_function: fn(&str) -> bool,
}

/// `node` runs inside a try body or its handler within the same function.
pub(crate) fn inside_try(node: Node, syntax: &TrySyntax) -> bool {
    let mut child = node;
    while let Some(parent) = child.parent() {
        if (syntax.is_function)(parent.kind()) {
            return false;
        }
        let protected = child.kind() == syntax.handler_kind
            || parent
                .child_by_field_name("body")
                .is_some_and(|body| body.id() == child.id());
        if parent.kind() == syntax.try_kind && protected {
            return true;
        }
        child = parent;
    }
    false
}

/// The handler neither rethrows nor uses `error` beyond passing it to a logger.
pub(crate) fn swallows(body: Node, error: Option<&str>, src: &[u8], syntax: &TrySyntax) -> bool {
    let mut stack = vec![(body, false)];
    while let Some((node, logged)) = stack.pop() {
        if syntax.throw_kinds.contains(&node.kind()) {
            return false;
        }
        if (syntax.is_function)(node.kind()) {
            continue;
        }
        let reads_error =
            node.child_count() == 0 && error.is_some_and(|name| node.utf8_text(src) == Ok(name));
        if reads_error && !logged {
            return false;
        }
        let callee = logger_callee(node, src, syntax);
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor).map(|child| {
            let argument = callee.is_some_and(|callee| callee.id() != child.id());
            (child, logged || argument)
        }));
    }
    true
}

fn logger_callee<'a>(node: Node<'a>, src: &[u8], syntax: &TrySyntax) -> Option<Node<'a>> {
    (node.kind() == syntax.call_kind)
        .then(|| node.child_by_field_name("function"))
        .flatten()
        .filter(|callee| {
            let text = callee.utf8_text(src).unwrap_or_default();
            syntax.loggers.iter().any(|p| matches_callee(p, text))
        })
}

fn matches_callee(pattern: &str, text: &str) -> bool {
    match (pattern.strip_prefix('*'), pattern.strip_suffix('*')) {
        (Some(suffix), _) => text.ends_with(suffix),
        (_, Some(prefix)) => text.starts_with(prefix),
        _ => text == pattern,
    }
}
