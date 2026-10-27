//! Shared try/handler shapes behind defensive-fallback facts.

use crate::spine::ir::HandlerEnd;
use tree_sitter::Node;

/// Per-language try/handler syntax and logger callees.
pub(crate) struct TrySyntax {
    pub(crate) try_kind: &'static str,
    pub(crate) handler_kind: &'static str,
    pub(crate) throw_kinds: &'static [&'static str],
    pub(crate) call_kind: &'static str,
    /// Callee texts; `*` matches any run of characters.
    pub(crate) loggers: &'static [&'static str],
    /// Callees that show the failure to a user (`toast.error`, `setError`).
    pub(crate) notifiers: &'static [&'static str],
    pub(crate) is_function: fn(&str) -> bool,
    pub(crate) return_kind: &'static str,
    /// Statements that do nothing (`pass`, comments).
    pub(crate) noop_kinds: &'static [&'static str],
    pub(crate) comment_kind: &'static str,
    /// Blocks that run as cleanup after the try (`finally`).
    pub(crate) cleanup_kinds: &'static [&'static str],
    /// Callee texts that parse JSON.
    pub(crate) parse_calls: &'static [&'static str],
    /// Callees expected to throw as an answer (storage access, file-existence checks).
    pub(crate) probes: &'static [&'static str],
    /// Classifies a returned literal; `None` for anything else.
    pub(crate) returned_literal: fn(Node, &[u8]) -> Option<HandlerEnd>,
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

pub(crate) fn logger_callee<'a>(
    node: Node<'a>,
    src: &[u8],
    syntax: &TrySyntax,
) -> Option<Node<'a>> {
    (node.kind() == syntax.call_kind)
        .then(|| node.child_by_field_name("function"))
        .flatten()
        .filter(|callee| {
            let text = callee.utf8_text(src).unwrap_or_default();
            syntax.loggers.iter().any(|p| matches_callee(p, text))
        })
}

/// `text` matches `pattern`, where `*` stands for any run of characters.
pub(crate) fn matches_callee(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = text.strip_prefix(first) else {
        return false;
    };
    let mut parts: Vec<&str> = parts.collect();
    let Some(last) = parts.pop() else {
        return rest.is_empty();
    };
    for part in parts {
        let Some(at) = rest.find(part) else {
            return false;
        };
        rest = &rest[at + part.len()..];
    }
    rest.ends_with(last)
}
