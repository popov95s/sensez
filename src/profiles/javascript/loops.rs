//! JS/TS loop frames: loop kind, bindings, bound and per-item callbacks.

use crate::profiles::for_loop::{self, RangeEnd};
use crate::profiles::walk;
use crate::spine::ir::{LoopBound, LoopFrame, LoopKind};
use tree_sitter::Node;

const BINDING_KINDS: &[&str] = &["identifier", "shorthand_property_identifier_pattern"];
const ITERATION_CALLBACK_METHODS: &[&str] = &[
    "map", "flatMap", "forEach", "filter", "reduce", "some", "every",
];

/// Fields of `loop_node` that run once per iteration.
pub fn is_iterated_part(loop_node: Node, child: Node) -> bool {
    let fields: &[&str] = match loop_node.kind() {
        "for_statement" => &["condition", "increment", "body"],
        _ => &["body"],
    };
    walk::is_field_child(loop_node, child, fields)
}

pub fn loop_frame(node: Node, src: &[u8], parent: Option<usize>) -> LoopFrame {
    let field = |name| node.child_by_field_name(name);
    let text = |name| walk::loop_subject(node, src, &[name]).unwrap_or_default();
    let mut frame = LoopFrame {
        parent,
        ..LoopFrame::default()
    };
    match node.kind() {
        "for_in_statement" => {
            frame.kind = LoopKind::ForOf;
            frame.bindings = names(field("left"), src);
            frame.subject = text("right");
            if super::performance::is_bounded_loop(&frame.subject) {
                frame.bound = LoopBound::Fixed;
            }
        }
        "for_statement" => {
            frame.kind = LoopKind::ForIndex;
            frame.bindings = field("initializer")
                .map(|init| declared_names(init, src))
                .unwrap_or_default();
            frame.subject = text("condition");
            let start = field("initializer").map_or(0, |init| initial_value(init, src));
            frame.bound = index_bound(&frame.subject, start);
        }
        _ => {
            frame.kind = LoopKind::While;
            frame.subject = text("condition");
            frame.cond_idents = names(field("condition"), src);
        }
    }
    frame
}

/// Synthetic frame for a callback passed to `xs.map(...)` and friends.
pub fn iteration_callback_frame(func: Node, src: &[u8]) -> Option<LoopFrame> {
    let call = func
        .parent()
        .filter(|p| p.kind() == "arguments")?
        .parent()
        .filter(|p| p.kind() == "call_expression")?;
    let callee = call
        .child_by_field_name("function")
        .filter(|f| f.kind() == "member_expression")?;
    let method = callee
        .child_by_field_name("property")
        .and_then(|p| walk::node_text(p, src))?;
    if !ITERATION_CALLBACK_METHODS.contains(&method) {
        return None;
    }
    let params = func
        .child_by_field_name("parameters")
        .or_else(|| func.child_by_field_name("parameter"));
    Some(LoopFrame {
        kind: LoopKind::IterCallback,
        bindings: names(params, src),
        subject: walk::loop_subject(callee, src, &["object"]).unwrap_or_default(),
        ..LoopFrame::default()
    })
}

pub fn names(node: Option<Node>, src: &[u8]) -> Vec<String> {
    node.map(|n| walk::identifiers(n, src, BINDING_KINDS))
        .unwrap_or_default()
}

/// Names bound by the `variable_declarator`s under `node`.
pub fn declared_names(node: Node, src: &[u8]) -> Vec<String> {
    if node.kind() == "variable_declarator" {
        return names(node.child_by_field_name("name"), src);
    }
    let mut cursor = node.walk();
    let children: Vec<Node> = node.named_children(&mut cursor).collect();
    children
        .into_iter()
        .flat_map(|child| declared_names(child, src))
        .collect()
}

/// Literal start of `let i = N`, else 0.
fn initial_value(initializer: Node, src: &[u8]) -> usize {
    walk::node_text(initializer, src)
        .and_then(|text| text.split_once('='))
        .and_then(|(_, value)| value.trim().trim_end_matches(';').trim().parse().ok())
        .unwrap_or(0)
}

/// `i < N` / `i <= N` with a literal or constant-named `N`.
fn index_bound(condition: &str, start: usize) -> LoopBound {
    let (limit, range_end) = match condition.split_once("<=") {
        Some((_, limit)) => (limit, RangeEnd::Inclusive),
        None => match condition.split_once('<') {
            Some((_, limit)) => (limit, RangeEnd::Exclusive),
            None => return LoopBound::Unbounded,
        },
    };
    let limit = limit.trim().trim_end_matches(';').trim();
    if let Ok(end) = limit.parse::<usize>() {
        return LoopBound::Count(for_loop::trip_count(start, end, range_end));
    }
    if super::performance::is_bounded_loop(limit) {
        return LoopBound::Fixed;
    }
    LoopBound::Unbounded
}

#[cfg(test)]
mod tests {
    use super::index_bound;
    use crate::spine::ir::LoopBound;

    #[test]
    fn inclusive_bound_at_usize_max_saturates() {
        let bound = index_bound("i <= 18446744073709551615", 0);
        assert_eq!(bound, LoopBound::Count(usize::MAX));
    }

    #[test]
    fn index_bound_subtracts_literal_start() {
        assert_eq!(index_bound("i < 8", 5), LoopBound::Count(3));
    }
}
