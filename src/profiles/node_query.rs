//! Read-only queries over tree-sitter nodes shared by the walkers.

use crate::spine::ir::{PerfLine, PerformanceFacts};
use tree_sitter::Node;

pub(crate) fn node_text<'a>(node: Node, src: &'a [u8]) -> Option<&'a str> {
    node.utf8_text(src).ok()
}

pub(crate) fn perf_line(node: Node, src: &[u8], subject_fields: &[&str]) -> PerfLine {
    PerfLine {
        line: node.start_position().row + 1,
        subject: loop_subject(node, src, subject_fields).unwrap_or_default(),
        frame: None,
    }
}

/// Texts of every node under `node` (itself included) whose kind is in `kinds`.
pub(crate) fn identifiers(node: Node, src: &[u8], kinds: &[&str]) -> Vec<String> {
    if kinds.contains(&node.kind()) {
        return node_text(node, src)
            .map(|name| vec![name.to_string()])
            .unwrap_or_default();
    }
    let mut cursor = node.walk();
    let children: Vec<Node> = node.named_children(&mut cursor).collect();
    children
        .into_iter()
        .flat_map(|child| identifiers(child, src, kinds))
        .collect()
}

/// `identifier` texts under `node`'s `field` child.
pub(crate) fn field_identifiers(node: Node, src: &[u8], field: &str) -> Vec<String> {
    node.child_by_field_name(field)
        .map(|child| identifiers(child, src, &["identifier"]))
        .unwrap_or_default()
}

/// `value` contains a `call_kind` node or reads a loop local holding a call result.
pub(crate) fn fed_by_call(
    facts: &PerformanceFacts,
    frame: Option<usize>,
    value: Node,
    src: &[u8],
    call_kind: &str,
) -> bool {
    contains_kind(value, call_kind)
        || identifiers(value, src, &["identifier"])
            .iter()
            .any(|name| facts.is_call_result(frame, name))
}

fn contains_kind(node: Node, kind: &str) -> bool {
    let mut cursor = node.walk();
    let found = node.kind() == kind
        || node
            .named_children(&mut cursor)
            .any(|child| contains_kind(child, kind));
    found
}

/// `child` is `parent`'s value for one of `fields`.
pub(crate) fn is_field_child(parent: Node, child: Node, fields: &[&str]) -> bool {
    fields.iter().any(|field| {
        parent
            .child_by_field_name(field)
            .is_some_and(|node| node.id() == child.id())
    })
}

pub(crate) fn loop_subject(node: Node, src: &[u8], subject_fields: &[&str]) -> Option<String> {
    subject_fields
        .iter()
        .find_map(|field| node.child_by_field_name(field))
        .and_then(|n| node_text(n, src))
        .filter(|text| text.len() <= 120)
        .map(str::to_string)
}
