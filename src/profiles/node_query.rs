//! Read-only queries over tree-sitter nodes shared by the walkers.

use crate::profiles::node_names::NodeNames;
use crate::spine::ir::{ChainHit, PerfLine, PerformanceFacts};
use tree_sitter::Node;

/// Text after the last `.`: `pkg.mod.Name` → `Name`.
pub(crate) fn last_segment(path: &str) -> &str {
    path.rsplit_once('.').map_or(path, |(_, last)| last)
}

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
    if kinds.contains(&node.kind_name()) {
        return node_text(node, src)
            .map(|name| vec![name.to_string()])
            .unwrap_or_default();
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .flat_map(|child| identifiers(child, src, kinds))
        .collect();
    found
}

/// `identifier` texts under `node`'s `field` child.
pub(crate) fn field_identifiers(node: Node, src: &[u8], field: &str) -> Vec<String> {
    node.field(field)
        .map(|child| identifiers(child, src, &["identifier"]))
        .unwrap_or_default()
}

/// Inside a loop, `value` contains a `call_kind` node or reads a loop local holding a call result.
pub(crate) fn fed_by_call(
    facts: &PerformanceFacts,
    frame: Option<usize>,
    value: Node,
    src: &[u8],
    call_kind: &str,
) -> bool {
    frame.is_some()
        && (contains_kind(value, call_kind)
            || identifiers(value, src, &["identifier"])
                .iter()
                .any(|name| facts.is_call_result(frame, name)))
}

fn contains_kind(node: Node, kind: &str) -> bool {
    let mut cursor = node.walk();
    let found = node.kind_name() == kind
        || node
            .named_children(&mut cursor)
            .any(|child| contains_kind(child, kind));
    found
}

/// `child` is `parent`'s value for one of `fields`.
pub(crate) fn is_field_child(parent: Node, child: Node, fields: &[&str]) -> bool {
    fields.iter().any(|field| {
        parent
            .field(field)
            .is_some_and(|node| node.id() == child.id())
    })
}

/// Longest loop subject text kept for display.
const MAX_SUBJECT_BYTES: usize = 120;

pub(crate) fn loop_subject(node: Node, src: &[u8], subject_fields: &[&str]) -> Option<String> {
    subject_fields
        .iter()
        .find_map(|field| node.field(field))
        .and_then(|n| node_text(n, src))
        .filter(|text| text.len() <= MAX_SUBJECT_BYTES)
        .map(str::to_string)
}

/// One grammar's member-access node: its kind, object field and chain-root kinds.
pub(crate) struct ChainShape {
    pub(crate) member_kind: &'static str,
    pub(crate) object_field: &'static str,
    pub(crate) root_kinds: &'static [&'static str],
}

/// The chain hit for the outermost member of a pure `a.b.c.d` access; inner members yield `None`.
pub(crate) fn chain_hit(
    node: Node,
    parent: Option<Node>,
    src: &[u8],
    shape: &ChainShape,
) -> Option<ChainHit> {
    let is_inner = parent.is_some_and(|p| {
        p.kind_name() == shape.member_kind
            && p.field(shape.object_field).map(|o| o.id()) == Some(node.id())
    });
    if is_inner {
        return None;
    }
    Some(ChainHit {
        line: node.start_position().row + 1,
        depth: chain_len(node, shape),
        root: chain_root(node, src, shape),
    })
}

fn chain_len(node: Node, shape: &ChainShape) -> usize {
    match node.field(shape.object_field) {
        Some(obj) if obj.kind_name() == shape.member_kind => chain_len(obj, shape) + 1,
        _ => 1,
    }
}

fn chain_root(node: Node, src: &[u8], shape: &ChainShape) -> Option<String> {
    let obj = node.field(shape.object_field)?;
    if obj.kind_name() == shape.member_kind {
        return chain_root(obj, src, shape);
    }
    shape
        .root_kinds
        .contains(&obj.kind_name())
        .then(|| node_text(obj, src).map(str::to_string))
        .flatten()
}
