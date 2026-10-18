//! Read-only queries over tree-sitter nodes shared by the walkers.

use crate::spine::ir::PerfLine;
use tree_sitter::Node;

pub(crate) fn node_text<'a>(node: Node, src: &'a [u8]) -> Option<&'a str> {
    node.utf8_text(src).ok()
}

pub(crate) fn perf_line(node: Node, src: &[u8], subject_fields: &[&str]) -> PerfLine {
    PerfLine {
        line: node.start_position().row + 1,
        subject: loop_subject(node, src, subject_fields).unwrap_or_default(),
    }
}

fn loop_subject(node: Node, src: &[u8], subject_fields: &[&str]) -> Option<String> {
    subject_fields
        .iter()
        .find_map(|field| node.child_by_field_name(field))
        .and_then(|n| node_text(n, src))
        .filter(|text| text.len() <= 120)
        .map(str::to_string)
}
