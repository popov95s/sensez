//! How a swallowing error handler ends, for `swallowed_error_sentinel` facts.

use super::try_handling::{logger_callee, matches_callee, swallows, TrySyntax};
use crate::profiles::node_names::NodeNames;
use crate::spine::ir::{GuardedBody, HandlerComment, HandlerEnd, SentinelHandler};
use tree_sitter::Node;

/// A handler whose `body` drops `error` and ends with a placeholder or a log.
pub(crate) fn sentinel_handler(
    handler: Node,
    body: Node,
    error: Option<&str>,
    src: &[u8],
    syntax: &TrySyntax,
    frame: Option<usize>,
) -> Option<SentinelHandler> {
    let try_body = handler.parent()?.field("body")?;
    if inside_cleanup(handler, syntax)
        || calls_any(body, syntax.notifiers, src, syntax)
        || calls_any(try_body, syntax.probes, src, syntax)
    {
        return None;
    }
    let (end, literal) = handler_end(body, error, src, syntax)?;
    Some(SentinelHandler {
        line: handler.start_position().row + 1,
        end,
        literal,
        guarded: guarded_body(try_body, src, syntax),
        comment: comment(body, syntax),
        frame,
    })
}

fn handler_end(
    body: Node,
    error: Option<&str>,
    src: &[u8],
    syntax: &TrySyntax,
) -> Option<(HandlerEnd, String)> {
    if !swallows(body, error, src, syntax) {
        return None;
    }
    let statements = statements(body, syntax);
    let log_only = || (HandlerEnd::LogOnly, String::new());
    let Some((last, before)) = statements.split_last() else {
        return Some(log_only());
    };
    let returned = (last.kind_name() == syntax.return_kind).then(|| last.named_child(0));
    match returned {
        Some(Some(value)) => (syntax.returned_literal)(value, src)
            .map(|end| (end, value.utf8_text(src).unwrap_or_default().to_string())),
        Some(None) => only_logs(before, src, syntax).then(log_only),
        None => only_logs(&statements, src, syntax).then(log_only),
    }
}

/// Cleanup code inside another handler or a `finally` block.
fn inside_cleanup(handler: Node, syntax: &TrySyntax) -> bool {
    std::iter::successors(handler.parent(), Node::parent)
        .take_while(|node| !(syntax.is_function)(node.kind_name()))
        .any(|node| {
            node.kind_name() == syntax.handler_kind
                || syntax.cleanup_kinds.contains(&node.kind_name())
        })
}

fn calls_any(root: Node, patterns: &[&str], src: &[u8], syntax: &TrySyntax) -> bool {
    descendants(root).any(|node| {
        callee_text(node, src, syntax).is_some_and(|callee| {
            patterns
                .iter()
                .any(|pattern| matches_callee(pattern, callee))
        })
    })
}

fn guarded_body(try_body: Node, src: &[u8], syntax: &TrySyntax) -> GuardedBody {
    let parses = |statement: &Node| {
        descendants(*statement).any(|node| {
            callee_text(node, src, syntax)
                .is_some_and(|callee| syntax.parse_calls.contains(&callee))
        })
    };
    match statements(try_body, syntax).as_slice() {
        [only] if parses(only) => GuardedBody::SingleParse,
        _ => GuardedBody::Other,
    }
}

fn comment(body: Node, syntax: &TrySyntax) -> HandlerComment {
    let mut cursor = body.walk();
    let commented = body
        .named_children(&mut cursor)
        .any(|node| node.kind_name() == syntax.comment_kind);
    if commented {
        HandlerComment::Present
    } else {
        HandlerComment::Absent
    }
}

fn statements<'a>(block: Node<'a>, syntax: &TrySyntax) -> Vec<Node<'a>> {
    let mut cursor = block.walk();
    block
        .named_children(&mut cursor)
        .filter(|node| {
            node.kind_name() != syntax.comment_kind
                && !syntax.noop_kinds.contains(&node.kind_name())
        })
        .collect()
}

fn only_logs(statements: &[Node], src: &[u8], syntax: &TrySyntax) -> bool {
    statements.iter().all(|statement| {
        statement.kind_name() == "expression_statement"
            && statement
                .named_child(0)
                .is_some_and(|call| logger_callee(call, src, syntax).is_some())
    })
}

fn callee_text<'a>(node: Node, src: &'a [u8], syntax: &TrySyntax) -> Option<&'a str> {
    (node.kind_name() == syntax.call_kind)
        .then(|| node.field("function"))
        .flatten()
        .and_then(|callee| callee.utf8_text(src).ok())
}

fn descendants(root: Node) -> impl Iterator<Item = Node> {
    let mut stack = vec![root];
    std::iter::from_fn(move || {
        let node = stack.pop()?;
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
        Some(node)
    })
}
